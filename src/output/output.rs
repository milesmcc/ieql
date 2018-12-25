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
    Partial,
}

#[derive(Debug)]
pub enum OutputItem {
    Url(Option<String>),
    Mime(Option<String>),
    Domain(Option<String>),
    Excerpt(Vec<PatternMatch>),
}

pub struct OutputBatch {
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
    pub fn new(
        document: &Document,
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
                ResponseItem::Domain => items.push(OutputItem::Domain(document.domain())),
                ResponseItem::Mime => {
                    items.push(OutputItem::Mime(string_clone_helper(&document.mime)))
                }
                ResponseItem::Url => {
                    items.push(OutputItem::Url(string_clone_helper(&document.url)))
                }
                ResponseItem::Excerpt => items.push(OutputItem::Excerpt(matches.clone())),
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
    pub fn merge_with(&mut self, other: OutputBatch) {
        self.outputs.extend(other.outputs);
    }

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