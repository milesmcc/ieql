pub trait Validatable {
    fn validate(&self) -> Option<Vec<Issue>>;
}

pub enum Issue {
    Warning(String),
    Error(String),
}