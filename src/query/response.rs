//! This file provides functionality related to responses.

use common::validation::{Issue, Validatable};

/// Represents a response—in other words, the parameters for
/// outputs.
///
/// This type _does not compile_ as it has 'no moving
/// parts'—it is simply information that is passed along
/// to the scanning system to guide it as it generates
/// outputs.
///
/// **This type is parallel, but _different_, from the
/// `Output` type.** You can think of a `Response` as the
/// _parameters_ for _creating_ Outputs; _not_ as the outputs
/// themselves.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Response {
    /// Represents the _kind_ of the response, which corresponds
    /// to the possible types of `Output`s: `Full` and `Partial`.
    ///
    /// For more information about `Full` and `Partial` responses,
    /// see the documentation for `ResponseKind` and `Output`.
    pub kind: ResponseKind,
    /// Represents the type of information that should be included
    /// in the `Output`.
    ///
    /// For more information about response items, see the
    /// documentation for `ResponseItem` and `OutputItem`.
    pub include: Vec<ResponseItem>,
}

/// Represents the kind of output that should be produced by the
/// query.
///
/// There is one-to-one parity between this type and `output::OutputKind`.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ResponseKind {
    Full,
    Partial,
}

/// Represents an item type that should be included in the query output.
///
/// There is one-to-one parity between this type and `output::OutputItem`.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ResponseItem {
    /// Denotes the URL of the document that matched the query, if present, should be included.
    /// There is _no guarantee_ that this will be a valid url; if the mechanism
    /// by which documents are loaded provides a faulty or invalid URL (such as
    /// a local filepath, as the command line interface does when loading documents),
    /// this URL will be unchanged.
    Url,
    /// Denotes that a valid IETF MIME type, as per RFC 2045, should be included.
    Mime,
    /// Denotes that the domain (or hostname) of the `Url` should be included.
    Domain,
    /// Denotes that any number of `PatternMatch`es—in other words, excerpts—should be included.
    Excerpt,
    /// Denotes that the full content of the web page should be included
    FullContent,
}

impl Validatable for Response {
    /// Validates the Response and ensures that no invalid parameters 
    /// are present.
    /// 
    /// More specifically, this function ensures that `Excerpt` and `Url`,
    /// which are not reducable, are not present in `include`.
    fn validate(&self) -> Option<Vec<Issue>> {
        let mut issues: Vec<Issue> = Vec::new();
        if self.kind == ResponseKind::Partial {
            let disallowed_items = vec![ResponseItem::Excerpt, ResponseItem::Url];
            for item in &self.include {
                if disallowed_items.contains(&item) {
                    issues.push(Issue::Error(format!(
                        "include `{:?}` is not allowed in partial responses",
                        item
                    )))
                }
            }
        }
        if issues.len() == 0 {
            return None;
        } else {
            return Some(issues);
        }
    }
}
