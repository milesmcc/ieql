extern crate ieql;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate rand;
extern crate ron;
extern crate simplelog;
extern crate walkdir;

use ieql::common::compilation::CompilableTo;
use ieql::common::retrieve::load_document;
use ieql::common::validation::{Issue, Validatable};
use ieql::input::document::{Document, DocumentBatch, DocumentReference,
    DocumentReferenceBatch,
};
use ieql::ScopeContent;
use ieql::output::output::OutputBatch;
use ieql::query::query::{Query, QueryGroup};
use ieql::scan::scanner::{Scanner, AsyncScanInterface};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

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
                .arg_from_usage("-t, --threads=[# of threads] 'If multithreading, how many threads to use'")
                .arg_from_usage("-h, --hide-outputs 'Do not show outputs'")
                .arg_from_usage("-R, --recursive 'Enter directories recursively'")
                .arg_from_usage("-o, --output=[dir] 'Directory to place outputs")
                .args_from_usage("-p, --pretty 'Pretty-print output files'"),
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
                return QueryGroup { queries: vec![], optimized_content: ScopeContent::Raw };
            }
        });
    }
    QueryGroup { queries: queries, optimized_content: ScopeContent::Raw }
}

fn write_output_batch_to_file(
    parent_directory: &str,
    output_batch: &OutputBatch,
    pretty: bool,
) -> bool {
    let dir_path = Path::new(&parent_directory);
    if !dir_path.is_dir() {
        error!(
            "output location `{}` is not a directory",
            dir_path.to_string_lossy()
        );
        return false;
    }
    for output in &output_batch.outputs {
        let query_id = match &output.query_id {
            Some(value) => value.clone(),
            None => String::from("unknown_query"),
        };
        let output_filename = match &output.id {
            Some(value) => format!("output-{}-{}.ieqlo", value, query_id),
            None => format!("output-{}-{}.ieqlo", rand::random::<u32>(), query_id),
        };
        let file_path = dir_path.join(output_filename.clone());
        let output_string = match match pretty {
            // shhh... this is fine...
            true => ron::ser::to_string_pretty(&output, ron::ser::PrettyConfig::default()),
            false => ron::ser::to_string(&output),
        } {
            Ok(value) => value,
            Err(error) => {
                error!(
                    "unable to serialize output `{}` (`{}`), skipping...",
                    output_filename, error
                );
                continue;
            }
        };
        match fs::write(file_path, output_string) {
            Ok(_) => (),
            Err(error) => error!("unable to write output `{}` (`{}`)", output_filename, error),
        }
    }
    return true;
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
        Ok(_value) => info!("query compiled successfully"),
        Err(error) => error!("unable to compile query: `{}`", error),
    }
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
    let threads: u8 = match matches.value_of("threads").unwrap_or("8").parse() {
        Ok(value) => value,
        Err(error) => {
            error!("invalid number of threads `{}` (`{}`), defaulting to 8...", matches.value_of("threads").unwrap(), error);
            8
        }
    };
    let hide_outputs = matches.is_present("hide-outputs");
    let recursive = matches.is_present("recursive");
    let should_output = matches.is_present("output");
    let output_dir = matches.value_of("output").unwrap_or("/tmp/"); // will not be used unless `should_output` is true
    let pretty_output = matches.is_present("pretty");
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

    match multithreaded {
        true => {
            let batch_size = 64;
            let mut async_interface: AsyncScanInterface = compiled_queries.scan_concurrently(threads);
            info!("will perform scan using {} threads", threads);
            let mut current_documents: Vec<DocumentReference> = Vec::new();
            for file_path_box in files_to_scan {
                let file_path = Box::leak(file_path_box);
                let document_reference = DocumentReference::Unpopulated(match file_path.to_str() {
                    Some(value) => String::from(value),
                    None => {
                        error!(
                            "unable to handle file `{}`, skipping...",
                            file_path.to_string_lossy()
                        );
                        continue;
                    }
                }); // TODO: will the lossyness ever be an issue?
                current_documents.push(document_reference);
                let num_documents = current_documents.len();
                if num_documents >= batch_size {
                    // time to push a batch
                    let mut drain: Vec<DocumentReference> = Vec::new();
                    drain.extend(current_documents.drain(0..batch_size));
                    let len = drain.len();
                    let batch = DocumentReferenceBatch::from(drain);
                    match async_interface.process(batch) {
                        Ok(_) => {
                            debug!("sending new batch of {} documents", len);
                        }
                        Err(_) => {
                            error!("unable to transmit batch to scan engine; shutting down...");
                            break;
                        }
                    };
                }
            }
            if current_documents.len() != 0 {
                // send all other documents
                let batch = DocumentReferenceBatch::from(current_documents);
                match async_interface.process(batch) {
                    Ok(_) => {
                        debug!("sending final batch");
                    }
                    Err(_) => {
                        error!("unable to transmit batch to scan engine; shutting down...");
                    }
                };
            }
            let mut output_batch = OutputBatch::new();
            (&mut async_interface).shutdown();
            loop {
                match async_interface.lock_for_outputs() {
                    Ok(value) => {
                        if !hide_outputs {
                            for output in &value.outputs {
                                info!("  - {}", output);
                            }
                        }
                        if should_output {
                            write_output_batch_to_file(output_dir, &value, pretty_output);
                        }
                        output_batch.merge_with(value);
                    }
                    Err(_) => break,
                }
            }
            info!("{} currently processing", async_interface.batches_pending_processing());
            info!(
                "finished scan and received {} output(s)",
                output_batch.outputs.len()
            );
            if should_output {
                info!("wrote outputs to `{}`", output_dir);
            }
        }
        false => {
            info!("performing single-threaded scan...");
            warn!("single-threaded scans load all files into memory before performing the scan");
            warn!("for a more performant alternative, run with `--multithreading`");
            let mut documents: Vec<Document> = Vec::new();
            for file_path_box in files_to_scan {
                let file_path = Box::leak(file_path_box);
                let file_path_str = match file_path.to_str() {
                    Some(value) => String::from(value),
                    None => {
                        error!(
                            "unable to handle file `{}`, skipping...",
                            file_path.to_string_lossy()
                        );
                        continue;
                    }
                };
                match load_document(&file_path_str) {
                    Ok(document) => documents.push(document),
                    Err(error) => {
                        error!(
                            "unable to process `{}` (`{}`), skipping...",
                            file_path_str, error
                        );
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
            if !hide_outputs {
                for output in &output_batch.outputs {
                    info!("  - {}", output);
                }
            }
            if should_output {
                write_output_batch_to_file(output_dir, &output_batch, pretty_output);
                info!("wrote outputs to `{}`", output_dir);
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
    let query_str = match std::str::from_utf8(contents.as_slice()) {
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
