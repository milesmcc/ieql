use query::scope::{CompiledScope, ScopeContent};
use scraper::Html;
use std::collections::HashMap;
use url::Url;
use common::compilation::CompilableTo;
use common::validation::Issue;

pub struct Document {
    pub url: Option<String>,
    pub data: Vec<u8>,
    pub mime: Option<String>,
}

pub struct CompiledDocument {
    pub url: Option<String>,
    pub raw: String,
    pub mime: Option<String>,
    pub text: String,
    pub domain: Option<String>,
}

pub struct DocumentBatch {
    pub documents: Vec<Document>,
}

pub struct CompiledDocumentBatch {
    pub documents: Vec<CompiledDocument>,
}

enum DocumentKind {
    Html,
    Unknown
}

impl Document {
    fn detect_document_kind(&self) -> DocumentKind {
        // Detect HTML
        let mut is_html = match &self.mime {
            Some(value) => value.eq("text/html"),
            None => false,
        };
        match &self.url {
            Some(value) => {
                if value.ends_with(".html") {
                    is_html = true;
                }
            }
            None => (),
        };
        if is_html {
            return DocumentKind::Html;
        }

        DocumentKind::Unknown
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

    fn raw(&self) -> String {
        String::from_utf8_lossy(self.data.as_slice()).into_owned()
    }

    fn extract_document_text(&self) -> String {
        match &self.detect_document_kind() {
            DocumentKind::Html => {
                let document = Html::parse_fragment(self.raw().as_str());
                let words = document.root_element().text().collect::<Vec<_>>();
                let text = words.join(" ");
                text
            }
            DocumentKind::Unknown => self.raw(),
        }
    }
}

impl CompilableTo<CompiledDocument> for Document {
    fn compile(&self) -> Result<CompiledDocument, Issue> {
        let text = self.extract_document_text();
        let domain = self.domain();
        let raw = self.raw();
        Ok(CompiledDocument {
            url: self.url.clone(),
            raw: raw,
            mime: self.mime.clone(),
            text: text,
            domain: domain
        })
    }
}

impl CompilableTo<CompiledDocumentBatch> for DocumentBatch {
    fn compile(&self) -> Result<CompiledDocumentBatch, Issue> {
        let mut compiled_documents: Vec<CompiledDocument> = Vec::new();
        for document in &self.documents {
            let compiled_document = match document.compile() {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            compiled_documents.push(compiled_document);
        }
        Ok(CompiledDocumentBatch {
            documents: compiled_documents
        })
    }
}

impl CompiledDocument {
    pub fn content(&self, content: ScopeContent) -> &String {
        match content {
            ScopeContent::Raw => &self.raw,
            ScopeContent::Text => &self.text,
        }
    }
}

impl From<Vec<Document>> for DocumentBatch {
    fn from(docs: Vec<Document>) -> DocumentBatch {
        DocumentBatch { documents: docs }
    }
}
