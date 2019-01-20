//! This file provides functionality related to scopes.

use common::pattern::{CompiledPattern, Pattern};
use common::compilation::CompilableTo;
use common::validation::Issue;

/// A `Scope` describes the kind of data that will be passed
/// to the queries, and which queries will be invoked.
/// 
/// The name describes exactly what it suggests: the _scope_
/// of a particular query.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Scope {
    /// The scope's `Pattern` is applied to each URL being passed
    /// through the scan engine. In order for a query to be run on
    /// a particular piece of data, the pattern must match
    /// that data's URL.
    pub pattern: Pattern,
    /// The content defines the type of content that the query's
    /// triggers will be run on. (Possible options include `Raw`
    /// and `Text`; for more information, see `ScopeContent`.)
    pub content: ScopeContent
}

#[derive(Clone)]
pub struct CompiledScope {
    pub pattern: CompiledPattern,
    pub content: ScopeContent
}

/// Denotes a form of text data to be passed to the query.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ScopeContent {
    /// The raw data—usually either HTML or utf8 extracted from
    /// web data/PDFs.
    Raw,
    /// Intelligently extracted text from the document. For HTML
    /// documents, for example, the `Text` is found by passing the
    /// content through an HTML engine and extracting _all_ the text.AsMut
    /// 
    /// Note that sometimes JavaScript text is also included.
    Text
}

impl CompilableTo<CompiledScope> for Scope {
    fn compile(&self) -> Result<CompiledScope, Issue> {
        match self.pattern.compile() {
            Ok(compiled_pattern) => Ok(CompiledScope {
                pattern: compiled_pattern,
                content: self.content,
            }),
            Err(issue) => Err(issue)
        }
    }
}