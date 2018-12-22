use url::Url;
use std::collections::HashMap;
use query::scope::{CompiledScope, ScopeContent};

pub struct Document {
    pub url: Option<String>,
    pub raw: String,
    pub mime: Option<String>,
    cache: HashMap<String, String>,
}

pub struct DocumentBatch {
    pub documents: Vec<Document>,
}

impl Document {
    pub fn new(url: Option<String>, raw: String, mime: Option<String>) -> Document {
        Document {
            url: url,
            raw: raw,
            mime: mime,
            cache: HashMap::new(),
        }
    }

    // Same as host, but `domain` is more understandable and common
    pub fn domain(&self) -> Option<String> {
        let own_url = match &self.url {
            Some(value) => value,
            None => return None,
        };
        let parsed_url = match Url::parse(own_url.as_str()) {
            Ok(url) => url,
            Err(_) => return None,
        };
        match parsed_url.host_str() {
            Some(value) => Some(String::from(value)),
            None => None,
        }
    }

    pub fn text(&self) -> &String {
        unimplemented!();
    }

    pub fn content(&self, content: ScopeContent) -> &String {
        match content {
            ScopeContent::Raw => &self.raw,
            ScopeContent::Text => self.text(),
        }
    }
}

impl From<Vec<Document>> for DocumentBatch {
    fn from(docs: Vec<Document>) -> DocumentBatch {
        DocumentBatch { documents: docs }
    }
}
