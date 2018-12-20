use common::pattern::PatternMatch;
use input::document::Document;
use query::query::CompiledQuery;
use query::response::{ResponseItem, ResponseKind};

pub struct Output {
    pub items: Vec<OutputItem>,
    pub kind: OutputKind,
    pub id: Option<String>,
    pub query_id: Option<String>,
}

pub enum OutputKind {
    Full,
    Partial
}

pub enum OutputItem {
    Url(Option<String>),
    Mime(Option<String>),
    Domain(Option<String>),
    Excerpt(Vec<PatternMatch>),
}

pub struct OutputBatch {
    pub outputs: Vec<Output>
}

impl From<Vec<Output>> for OutputBatch {
    fn from(outputs: Vec<Output>) -> OutputBatch {
        OutputBatch {
            outputs: outputs
        }
    }
}

fn string_clone_helper(to_clone: &Option<String>) -> Option<String> {
    match to_clone {
        Some(value) => Some(value.clone()),
        None => None
    }
}

pub fn assemble(document: &Document, query: &CompiledQuery, matches: Vec<PatternMatch>, id: Option<String>) -> Output {
    // warning: expensive!
    let kind = match query.response.kind {
        ResponseKind::Full => OutputKind::Full,
        ResponseKind::Partial => OutputKind::Partial,
    };
    let query_id = string_clone_helper(&query.id);
    let mut items: Vec<OutputItem> = Vec::new();
    for item in &query.response.include {
        match item {
            ResponseItem::Domain => items.push(OutputItem::Domain(document.domain())),
            ResponseItem::Mime => items.push(OutputItem::Mime(string_clone_helper(&document.mime))),
            ResponseItem::Url => items.push(OutputItem::Domain(string_clone_helper(&document.url))),
            ResponseItem::Excerpt => items.push(OutputItem::Excerpt(matches.clone()))
        }
    }
    Output {
        items: items,
        kind: kind,
        id: id,
        query_id: query_id
    }
}