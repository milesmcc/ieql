//! This file provides functionality related to outputs.

use common::pattern::PatternMatch;
use input::document::CompiledDocument;
use query::query::CompiledQuery;
use query::response::{ResponseItem, ResponseKind};

/// `Output` represents a 'match' of a Query. It is the primary
/// product of an IEQL scan, and contains many variable (and configurable)
/// pieces of data and metadata.
/// 
/// _Note: `response` and `output` are synonymous in the context of this
/// documentation._
/// 
/// Outputs have two kinds: `Full` and `Partial`. Full outputs are for
/// matches which exist on their own—for example, a match of someone's
/// name online. Partial outputs, however, are meant to be MapReduced.
/// For example, if a linguist wanted to count the number of times a certain
/// word appears online, they would configure their query to produce a `Partial`
/// response, which they would then MapReduce.
/// 
/// There is not currently full support for partial IEQL outputs.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Output {
    /// Contains the data relevant for the user; for example, excerpts of the match.
    pub items: Vec<OutputItem>,
    /// Represents the _kind_ of the output (`Full` or `Partial`).
    pub kind: OutputKind,
    /// This is an optional value for identifying the output, and can vary
    /// based on your implementation. In most cases, when present, this 
    /// is some form of UUID.
    pub id: Option<String>,
    /// This is the ID of the query that created the output. Note that this
    /// will only be present when the query that created the output itself
    /// has an id.
    pub query_id: Option<String>,
}

/// This enum specifies the output type of the query. For more information
/// about each type of query, please see the specification.
/// 
/// **There is currently not full support for partial queries.**
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum OutputKind {
    Full,
    Partial,
}

/// This enum represents a possible output item. `OutputItem`s are typically
/// user-meaningful, and are not machine readable. The items included in the
/// output are dependent on the `response` configuration of the query.
/// 
/// Much of this information is simply copied from the metadata of the document
/// that produced it.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum OutputItem {
    /// Represents the URL of the document that matched the query, if present.
    /// There is _no guarantee_ that this will be a valid url; if the mechanism
    /// by which documents are loaded provides a faulty or invalid URL (such as
    /// a local filepath, as the command line interface does when loading documents),
    /// this URL will be unchanged.
    Url(Option<String>),
    /// Represents a valid IETF MIME type, as per RFC 2045.
    Mime(Option<String>),
    /// Represents the domain (or hostname) of the `Url`. When the URL is not present, neither
    /// will the domain be.
    Domain(Option<String>),
    /// Contains any number of `PatternMatch`es—in other words, excerpts.
    Excerpt(Vec<PatternMatch>),
    /// Contains the full content of the matched page
    FullContent(Option<String>)
}

/// Represents a batch (collection) of outputs. This function tends to be
/// helpful for multiprocessing, though it is somewhat infrequently used.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OutputBatch {
    /// Contains the outputs.
    pub outputs: Vec<Output>,
}

impl From<Vec<Output>> for OutputBatch {
    fn from(outputs: Vec<Output>) -> OutputBatch {
        OutputBatch { outputs: outputs }
    }
}

fn string_clone_helper(to_clone: &Option<String>) -> Option<String> {
    match to_clone {
        Some(value) => Some(value.clone()),
        None => None,
    }
}

impl Output {
    /// Create a new output from the given data. Please note that this
    /// operation is **expensive**!
    /// 
    /// # Arguments
    /// * `document`: the compiled document that the query matched
    /// * `query`: the compiled query that matched the document
    /// * `matches`: the `PatternMatch`es produced by the queries' triggers
    /// * `id`: the optional ID of the desired output
    pub fn new(
        document: &CompiledDocument,
        query: &CompiledQuery,
        matches: Vec<PatternMatch>,
        id: Option<String>,
    ) -> Output {
        // warning: expensive!
        let kind = match query.response.kind {
            ResponseKind::Full => OutputKind::Full,
            ResponseKind::Partial => OutputKind::Partial,
        };
        let query_id = string_clone_helper(&query.id);
        let mut items: Vec<OutputItem> = Vec::new();
        for item in &query.response.include {
            match item {
                ResponseItem::Domain => items.push(OutputItem::Domain((&document.domain).clone())),
                ResponseItem::Mime => {
                    items.push(OutputItem::Mime(string_clone_helper(&document.mime)))
                }
                ResponseItem::Url => {
                    items.push(OutputItem::Url(string_clone_helper(&document.url)))
                }
                ResponseItem::Excerpt => items.push(OutputItem::Excerpt(matches.clone())),
                ResponseItem::FullContent => items.push(OutputItem::FullContent(Some((&document.raw).clone())))
            }
        }
        Output {
            items: items,
            kind: kind,
            id: id,
            query_id: query_id,
        }
    }
}

impl OutputBatch {
    /// Merges the output batch with the other output
    /// batch. This function _consumes_ the other output
    /// batch, but involves no memory duplication.
    pub fn merge_with(&mut self, other: OutputBatch) {
        self.outputs.extend(other.outputs);
    }

    /// Create a new empty output batch.
    pub fn new() -> OutputBatch {
        OutputBatch::from(vec![])
    }
}

impl std::fmt::Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let id = match &self.id {
            Some(value) => format!("[{}]", value),
            None => String::from(""),
        };
        let kind = match self.kind {
            OutputKind::Full => "full response",
            OutputKind::Partial => "partial response",
        };
        let query_id = match &self.query_id {
            Some(value) => format!(" from `{}`", value),
            None => String::from(""),
        };
        let mut items: Vec<String> = Vec::new();
        for item in &self.items {
            items.push(format!("{:?}", item));
        }
        write!(f, "{} {}{}: {:?}", id, kind, query_id, items)
    }}