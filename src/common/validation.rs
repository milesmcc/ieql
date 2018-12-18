pub trait Validatable {
    fn validate(&self) -> Option<Vec<Issue>>;
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Issue {
    Warning(String),
    Error(String),
}