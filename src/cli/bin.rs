extern crate ieql;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate ron;
extern crate simplelog;
extern crate walkdir;

use ieql::common::compilation::CompilableTo;
use ieql::common::validation::{Issue, Validatable};
use ieql::input::document::{CompiledDocument, CompiledDocumentBatch, Document, DocumentBatch};
use ieql::output::output::OutputBatch;
use ieql::query::query::{Query, QueryGroup};
use ieql::scan::scanner::Scanner;
use std::fs::DirEntry;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

use std::sync::mpsc;

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
                        .help(
                            "the path to the query, or a directory which contains multiple queries",
                        )
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
                .arg_from_usage("-R, --recursive 'Enter directories recursively'"),
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

fn get_queries_from_file(file: String) -> QueryGroup {
    let path = Path::new(&file);
    let mut queries: Vec<Query> = Vec::new();
    if path.is_dir() {
        for entry in WalkDir::new(path).follow_links(true).into_iter() {
            match entry {
                Ok(file) => {
                    if file.path().is_dir() {
                        continue;
                    }
                    let subpath: &Path = file.path();
                    let query = match get_query_from_file(subpath.to_string_lossy().into_owned()) {
                        Ok(value) => value,
                        Err(error) => {
                            warn!(
                                "unable to load query `{}` (`{}`), skipping...",
                                file.path().to_string_lossy(),
                                error
                            );
                            continue;
                        }
                    };
                    queries.push(query);
                }
                Err(error) => {
                    warn!("unable to handle nested query `{}`, skipping...", error);
                    continue;
                }
            }
        }
    } else {
        queries.push(match get_query_from_file(file.clone()) {
            Ok(value) => value,
            Err(error) => {
                error!("unable to load query `{}` (`{}`), skipping...", file, error);
                return QueryGroup { queries: vec![] };
            }
        });
    }
    QueryGroup { queries: queries }
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

fn load_document_from_file(file_path: &Path) -> Option<Document> {
    let mut f: File = match File::open(&file_path) {
        Ok(value) => value,
        Err(error) => {
            error!(
                "unable to open `{}` (`{}`), skipping...",
                file_path.to_string_lossy(),
                error
            );
            return None;
        }
    };
    let mut contents: Vec<u8> = Vec::new();
    match f.read_to_end(&mut contents) {
        Ok(size) => {}
        Err(error) => {
            error!(
                "unable to read `{}` (`{}`), skipping...",
                file_path.to_string_lossy(),
                error
            );
            return None;
        }
    }
    Some(Document {
        data: contents,
        mime: None,
        url: Some(String::from(file_path.to_string_lossy())),
    })
}

fn run_scan(matches: &clap::ArgMatches) {
    // Load queries
    let query_path = matches.value_of("query").unwrap();
    let file_paths: Vec<&str> = matches.values_of("inputs").unwrap().collect();
    let queries = get_queries_from_file(String::from(query_path));
    let compiled_queries = match queries.compile() {
        Ok(value) => {
            debug!("queries compiled successfully");
            value
        }
        Err(error) => {
            error!("unable to compile queries: `{}`", error);
            return;
        }
    };
    let multithreaded = matches.is_present("multithreading");
    let recursive = matches.is_present("recursive");
    let mut files_to_scan: Vec<Box<Path>> = Vec::new();
    for file_path in file_paths {
        let path = Path::new(file_path);
        if !path.exists() {
            warn!("unable to find file `{}`, skipping...", file_path);
            continue;
        }
        if path.is_dir() {
            if recursive {
                for entry in WalkDir::new(path).follow_links(true).into_iter() {
                    match entry {
                        Ok(file) => {
                            if file.path().is_dir() {
                                continue;
                            }
                            files_to_scan.push(Box::from(file.path()));
                        }
                        Err(error) => {
                            warn!("unable to handle nested file `{}`, skipping...", error);
                            continue;
                        }
                    }
                }
            } else {
                warn!(
                    "file `{}` is a directory, but recursion is not enabled; skipping...",
                    file_path
                );
                continue;
            }
        } else {
            files_to_scan.push(Box::from(path));
        }
    }
    info!(
        "scanning {} files with {} queries...",
        files_to_scan.len(),
        queries.queries.len()
    );

    // perform scan; note a caveat: loading files is _single threaded_
    match multithreaded {
        true => {
            let batch_size = 64;
            let (tx_batches, rx_batches) = mpsc::channel::<DocumentBatch>();
            let rx_outputs = compiled_queries.scan_concurrently(rx_batches, 8);

            let mut current_documents: Vec<Document> = Vec::new();
            for file_path_box in files_to_scan {
                let file_path = Box::leak(file_path_box);
                let document = match load_document_from_file(file_path) {
                    Some(value) => value,
                    None => continue,
                };
                current_documents.push(document);
                let num_documents = current_documents.len();
                if num_documents >= batch_size {
                    // time to push a batch
                    let mut drain: Vec<Document> = Vec::new();
                    drain.extend(current_documents.drain(batch_size..));
                    let batch = DocumentBatch::from(drain);
                    match tx_batches.send(batch) {
                        Ok(_) => {
                            debug!("sending new batch of {} documents", num_documents);
                        }
                        Err(_) => {
                            error!("unable to transmit batch to scan engine; shutting down...");
                            break;
                        }
                    };
                    current_documents = Vec::new();
                }
            }
            if current_documents.len() != 0 {
                // send all other documents
                let batch = DocumentBatch::from(current_documents);
                match tx_batches.send(batch) {
                    Ok(_) => {
                        debug!("sending final batch");
                    }
                    Err(_) => {
                        error!("unable to transmit batch to scan engine; shutting down...");
                    }
                };
            }
            drop(tx_batches);
            let mut output_batch = OutputBatch::new();
            for batch in rx_outputs {
                output_batch.merge_with(batch);
            }
            info!("received {} output(s)", output_batch.outputs.len());
            for output in output_batch.outputs {
                info!("  - {}", output);
            }
        }
        false => {
            info!("performing single-threaded scan...");
            warn!("single-threaded scans load all files into memory before performing the scan");
            warn!("for a more performant alternative, run with `--multithreading`");
            let mut documents: Vec<Document> = Vec::new();
            for file_path_box in files_to_scan {
                let file_path = Box::leak(file_path_box);
                match load_document_from_file(file_path) {
                    Some(document) => documents.push(document),
                    None => {
                        error!("unable to process `{}`...", file_path.to_string_lossy());
                        continue; // not strictly necessary but the verbosity is good
                    }
                }
            }
            let document_batch = match DocumentBatch::from(documents).compile() {
                Ok(value) => value,
                Err(error) => {
                    error!("unable to compile document batch: `{}`", error);
                    return;
                }
            };
            debug!("performing scan...");
            let output_batch = compiled_queries.scan_batch(&document_batch);
            info!("received {} output(s)", output_batch.outputs.len());
            for output in output_batch.outputs {
                info!("  - {}", output);
            }
        }
    }
}

fn get_query_from_file(path: String) -> Result<Query, Issue> {
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
