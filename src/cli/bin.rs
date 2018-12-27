extern crate ieql;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate ron;
extern crate simplelog;

use ieql::common::compilation::CompilableTo;
use ieql::common::validation::{Issue, Validatable};
use ieql::input::document::{Document, DocumentBatch};
use ieql::query::query::Query;
use ieql::scan::scanner::Scanner;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use clap::{App, Arg, SubCommand};

fn main() {
    simplelog::CombinedLogger::init(vec![simplelog::TermLogger::new(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
    )
    .unwrap()])
    .unwrap();

    let matches = App::new("IEQL Command Line Interface")
        .version(crate_version!())
        .about("Scan documents using the IEQL system.")
        .author(crate_authors!())
        .subcommand(
            SubCommand::with_name("validate")
                .about("Validate a given IEQL query")
                .arg(
                    Arg::with_name("query")
                        .help("the path of the IEQL query to validate")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("scan")
                .about("Scan documents using an IEQL query")
                .arg(
                    Arg::with_name("query")
                        .help("the path to the query")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("inputs")
                        .help("the path(s) to the input files")
                        .required(true)
                        .index(2)
                        .min_values(1),
                )
                .arg_from_usage("-m, --multithreading 'Scan using multiple CPU threads'")
                .arg_from_usage("-b, --binary 'Allow for binary files to be loaded'"),
        )
        .get_matches();
    run(matches);
}

fn run(matches: clap::ArgMatches) {
    match matches.subcommand() {
        ("validate", Some(m)) => run_validate(m),
        ("scan", Some(m)) => run_scan(m),
        _ => error!("no valid command specified; try running with `--help`."),
    }
}

fn run_validate(matches: &clap::ArgMatches) {
    // Adapted partially from my own software, https://github.com/milesmcc/ArmorLib/blob/master/src/cli/bin.rs

    let path: String = String::from(matches.value_of("query").unwrap()); // safe to unwrap, CLAP makes sure of it

    let query = match get_query_from_file(path) {
        Ok(value) => value,
        Err(issue) => {
            error!(
                "encountered a critical error while trying to load query: {}",
                issue
            );
            return;
        }
    };

    match query.validate() {
        Some(issues) => {
            error!("query validation encountered issues:");
            for issue in issues {
                error!("    - {}", issue);
            }
        }
        None => info!("validation encountered no errors"),
    }
    match query.compile() {
        Ok(value) => info!("query compiled successfully"),
        Err(error) => error!("unable to compile query: `{}`", error),
    }
}

fn run_scan(matches: &clap::ArgMatches) {
    let query_path = matches.value_of("query").unwrap();
    let file_paths: Vec<&str> = matches.values_of("inputs").unwrap().collect();
    let query = match get_query_from_file(String::from(query_path)) {
        Ok(value) => value,
        Err(error) => {
            error!(
                "encountered a critical error while trying to load query: {}",
                error
            );
            return;
        }
    };
    let compiled_query = match query.compile() {
        Ok(value) => {
            debug!("query compiled successfully");
            value
        }
        Err(error) => {
            error!("unable to compile query: `{}`", error);
            return;
        }
    };
    let multithreaded = matches.is_present("multithreading");
    let binary = matches.is_present("binary");
    if binary {
        warn!("binary files will be loaded; this may cause issues for some broad raw queries!");
    }
    match multithreaded {
        true => {
            error!("multi-threaded scanning is not yet supported!");
            unimplemented!();
        }
        false => {
            info!("performing single-threaded scan...");
            warn!("single-threaded scans load all files into memory before performing the scan");
            warn!("for a more performant alternative, run with `--multithreading`");
            let mut documents: Vec<Document> = Vec::new();
            for file_path in file_paths {
                let mut f: File = match File::open(file_path) {
                    Ok(value) => value,
                    Err(error) => {
                        error!("unable to open `{}` (`{}`), skipping...", file_path, error);
                        continue;
                    }
                };
                let mut contents: Vec<u8> = Vec::new();
                match f.read_to_end(&mut contents) {
                    Ok(size) => {},
                    Err(error) => {
                        error!("unable to read `{}` (`{}`), skipping...", file_path, error);
                        continue;
                    }
                }
                let document = Document {
                    data: contents,
                    mime: None,
                    url: Some(String::from(file_path))
                };
                documents.push(document);
            }
            let document_batch = match DocumentBatch::from(documents).compile() {
                Ok(value) => value,
                Err(error) => {
                    error!("unable to compile document batch: `{}`", error);
                    return;
                }
            };
            debug!("performing scan...");
            let output_batch = compiled_query.scan_batch(&document_batch);
            info!("received {} output(s)", output_batch.outputs.len());
            for output in output_batch.outputs {
                info!("  - {}", output);
            }
        }
    }
}

fn get_query_from_file(path: String) -> Result<Query, Issue> {
    info!("loading query file `{}`", path);

    if !path.ends_with(".ieql") {
        warn!("path does not end with `.ieql`")
    }

    let mut f = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            return Err(Issue::Error(format!("unable to open {}: {}", path, error)));
        }
    };
    let mut contents: Vec<u8> = Vec::new();
    match f.read_to_end(&mut contents) {
        Ok(size) => debug!("successfully read {} bytes", size),
        Err(error) => {
            return Err(Issue::Error(format!(
                "unable to read `{}`: `{}`",
                path, error
            )));
        }
    }
    let mut query_str = match std::str::from_utf8(contents.as_slice()) {
        Ok(value) => value,
        Err(error) => {
            return Err(Issue::Error(format!(
                "unable to convert file to string: `{}`",
                error
            )));
        }
    };
    let query: Query = match ron::de::from_str(query_str) {
        Ok(value) => value,
        Err(error) => {
            return Err(Issue::Error(format!(
                "unable to deserialize query: `{}`",
                error
            )));
        }
    };
    return Ok(query);
}
