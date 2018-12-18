pub trait Validatable {
    fn validate(&self) -> Option<Vec<Issue>>;
}

#[derive(Serialize, Deserialize)]
pub enum Issue {
    Warning(String),
    Error(String),
}