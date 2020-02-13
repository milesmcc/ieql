//! This document provides functionality related to document handling.

use common::compilation::CompilableTo;
use common::validation::Issue;
use query::scope::ScopeContent;
use regex::Regex;
use url::Url;
use lazy_static::lazy_static;
use htmlescape::decode_html;

lazy_static! {
    static ref HTML_REGEX: Regex = Regex::new(r"<(.*?)>").unwrap();
    static ref SPACE_REGEX: Regex = Regex::new(r"\s{2,}").unwrap();
}

/// The `Document` struct represents any kind of document, but typically
/// some sort of Internet document. A `Document` can often be quite large;
/// after all, it contains the entire text of a document.
///
/// In practice, this struct functions more as an interim format as data becomes
/// a `CompiledDocument`.
#[derive(Clone)]
pub struct Document {
    /// `url` represents the URL of the document, if it is present.
    ///
    /// For internet documents, this typically takes the form of `Some("https://...")`,
    /// whereas for local documents this typically takes the form of
    /// `Some("/path/to/file")`.
    pub url: Option<String>,
    /// `data` contains the data of the document.
    ///
    /// This data is stored as a `Vec<u8>` primarily for first-class text
    /// document support (`utf8`).
    pub data: Vec<u8>,
    /// `mime` represents a valid IETF `mime` type, as per RFC 2045.
    pub mime: Option<String>,
}

/// A `DocumentReference` is a reference to a document that is either
/// already loaded into memory or exists at some path. This path can,
/// in theory, be a URL or a relative (or absolute) path on the user's
/// local filesystem.
///
/// Currently, only local paths are supported. URLs will be supported
/// in a future version of IEQL.
///
/// The benefit of `DocumentReference` lies primarily in multithreading.
/// Using `DocumentReference`s allows for file IO to be parallelized.
/// (By passing a `DocumentReference` or `DocumentReferenceBatch` to
/// a concurrent scanner, one need not actually read the document from
/// the disk in the main thread.)
pub enum DocumentReference {
    /// Represents a document that is already present in memory and
    /// does not need to be loaded from the disk.
    Populated(Document),
    /// Represents a document that _has not already been loaded_. The
    /// contained `String` is the document's path.
    Unpopulated(String),
}

/// Represents a batch (collection in the form of a `Vec`) of
/// `DocumentReference`s.
///
/// This struct is particularly useful for scanning, as it allows
/// one function call to take many different document references.
/// It also enables 'processing groups'—i.e. groups of documents that
/// will always be processed together in the same thread.
pub struct DocumentReferenceBatch {
    /// Contains the DocumentReferences
    pub documents: Vec<DocumentReference>,
}

/// A `CompiledDocument` is a `Document` that has been processed and
/// is ready to be scanned. During compilation, the IEQL document compiler
/// extracts the following information from the `Document`:
///
/// * **text** — the text of the document. Currently, only HTML parsing is supported.
/// * **domain** — the domain name, if present, is also processed.
/// * **raw** — unlike `Documents`, whose contents are bytes, `CompiledDocuments` have text.
///
/// In cases that the document is not HTML, `text` is identical to `raw`.
pub struct CompiledDocument {
    pub url: Option<String>,
    pub raw: String,
    pub mime: Option<String>,
    pub text: String,
    pub domain: Option<String>,
}

/// Represents a batch (collection in the form of a `Vec`) of `Document`s.
pub struct DocumentBatch {
    /// Contains the documents
    pub documents: Vec<Document>,
}

/// Represents a batch (collection in the form of a `Vec`) of `CompiledDocument`s.
pub struct CompiledDocumentBatch {
    /// Contains the compiled documents
    pub documents: Vec<CompiledDocument>,
}

/// This enum represents the various kinds of documents which support intelligent
/// text extraction.
enum DocumentKind {
    Html,
    Unknown,
}

impl Document {
    /// This function detects the document's `DocumentKind` by looking at its path
    /// and MIME information.
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

    /// This function extracts the hostname (domain name) of a document. In cases where
    /// the host name isn't known, this function returns `None`.
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

    /// This function extracts text from the document's `data`. It assumes `utf8` encoding.
    /// Note that this function is very different from `extract_document_text()`: this function
    /// simply extracts text, while `extract_document_text()` also, in some cases, parses it.
    fn raw(&self) -> String {
        String::from_utf8_lossy(self.data.as_slice()).into_owned()
    }

    /// This function intelligently extracts text from the document—which is to say that it is
    /// able to parse HTML documents and extract the human-readable text. Additional document types,
    /// such as PDFs, will be supported in the future.
    fn extract_document_text(&self) -> String {
        match &self.detect_document_kind() {
            DocumentKind::Html => {
                let extracted = String::from(SPACE_REGEX.replace_all(&HTML_REGEX.replace_all(&self.raw(), " "), " "));
                match decode_html(extracted.as_str()) {
                    Ok(value) => value,
                    Err(_) => extracted
                }
            },
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
            domain: domain,
        })
    }
}

impl CompilableTo<CompiledDocumentBatch> for DocumentBatch {
    fn compile(&self) -> Result<CompiledDocumentBatch, Issue> {
        let mut compiled_documents: Vec<CompiledDocument> = Vec::new();
        for document in &self.documents {
            let compiled_document = match document.compile() {
                Ok(value) => value,
                Err(_error) => continue, // silent failure
            };
            compiled_documents.push(compiled_document);
        }
        Ok(CompiledDocumentBatch {
            documents: compiled_documents,
        })
    }
}

impl CompiledDocument {
    /// This function returns the document content relative to the
    /// given `ScopeContent`. For example, if the `ScopeContent`
    /// is `Raw`, this function will return the document's `Raw` data.
    /// If it is `Text`, this function will return the document's parsed
    /// text.
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

impl From<Vec<DocumentReference>> for DocumentReferenceBatch {
    fn from(docs: Vec<DocumentReference>) -> DocumentReferenceBatch {
        DocumentReferenceBatch { documents: docs }
    }
}
