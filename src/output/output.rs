pub struct Output {
    pub items: Vec<OutputItem>,
    pub kind: OutputKind,
    pub id: Option<String>,
    pub query: Option<String>,
}

pub enum OutputKind {
    Full,
    Partial
}

pub enum OutputItem {
    Url(String),
    Mime(String),
    Domain(String),
    Excerpt(String),
}