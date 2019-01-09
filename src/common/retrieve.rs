//! This file provides a utility class for loading files.

use input::document::Document;
use common::validation::Issue;
use std::fs::DirEntry;
use std::fs::File;
use std::path::Path;
use std::io::Read;

/// Loads the file at the given path and assembles a `Document`. This function
/// is a utility. It currently only supports local files.
/// 
/// # Arguments
/// * `path`: a `String` of the filepath to load
pub fn load_document(path: &String) -> Result<Document, Issue> {
    // TODO: make this work for more than just local file paths
    let file_path = Path::new(&path);
    let mut f: File = match File::open(&file_path) {
        Ok(value) => value,
        Err(error) => {
            return Err(Issue::Error(format!(
                "unable to open `{}` (`{}`), skipping...",
                file_path.to_string_lossy(),
                error
            )));
        },
    };
    let mut contents: Vec<u8> = Vec::new();
    match f.read_to_end(&mut contents) {
        Ok(_size) => {}
        Err(error) => {
            return Err(Issue::Error(format!(
                "unable to read `{}` (`{}`), skipping...",
                file_path.to_string_lossy(),
                error
            )));
        }
    }
    Ok(Document {
        data: contents,
        mime: None,
        url: Some(String::from(file_path.to_string_lossy())),
    })
}