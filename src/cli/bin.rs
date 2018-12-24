extern crate ieql;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate simplelog;
extern crate ron;

use clap::{App, Arg, SubCommand};

fn main() {
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(simplelog::LevelFilter::Info, simplelog::Config::default())
            .unwrap(),
    ]).unwrap();

    let matches = App::new("IEQL Command Line Interface")
        .version(crate_version!())
        .about("Scan documents using the IEQL system.")
        .author(crate_authors!())
        .subcommand(
            SubCommand::with_name("validate")
                .about("validate a given IEQL query")
                .arg(
                    Arg::with_name("query")
                        .help("the path of the IEQL query to validate")
                        .required(true)
                        .index(1),
                )
                // .arg(
                //     Arg::with_name("output_location")
                //         .help("Sets the path of the output file")
                //         .required(false)
                //         .index(2),
                // ),
        )
        // .subcommand(
        //     SubCommand::with_name("scan")
        //         .about("scan files and generate outputs"),
        //         .arg(
        //             Arg::with_name("queries")
        //                 .help("the path to the queries")
        //                 .required(true),
        //                 .index(1),
        //         )
        //         .arg
        // )
        .get_matches();
    run(matches);
}

fn run(matches: clap::ArgMatches) {
    match matches.subcommand() {
        ("validate", Some(m)) => run_validate(m),
        _ => error!("no command specified; try running with `--help`."),
    }
}

fn run_validate(matches: &clap::ArgMatches) {
    // Adapted partially from my own software, https://github.com/milesmcc/ArmorLib/blob/master/src/cli/bin.rs

    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use ieql::query::query::Query;
    use ieql::common::validation::Validatable;
    use ieql::common::compilation::CompilableTo;
    use ron;

    let path: String = String::from(matches.value_of("query").unwrap()); // safe to unwrap, CLAP makes sure of it

    info!("loading query file `{}`", path);

    if !path.ends_with(".ieql") {
        warn!("path does not end with `.ieql`")
    }

    let mut f = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            error!("unable to open {}: {}", path, error);
            return;
        }
    };
    let mut contents: Vec<u8> = Vec::new();
    match f.read_to_end(&mut contents) {
        Ok(size) => info!("successfully read {} bytes", size),
        Err(error) => {
            error!("unable to read `{}`: `{}`", path, error);
            return;
        }
    }
    let mut query_str = match std::str::from_utf8(contents.as_slice()) {
        Ok(value) => value,
        Err(error) => {
            error!("unable to convert file to string: `{}`", error);
            return;
        }
    };
    let query: Query = match ron::de::from_str(query_str) {
        Ok(value) => value,
        Err(error) => {
            error!("unable to deserialize query: `{}`", error);
            return;
        }
    };
    match query.validate() {
        Some(issues) => {
            error!("query validation encountered issues:");
            for issue in issues {
                error!("    - {}", issue);
            }
        },
        None => info!("validation encountered no errors"),
    }
    match query.compile() {
        Ok(value) => info!("query compiled successfully"),
        Err(error) => error!("unable to compile query: `{}`", error),
    }
}