use std::fmt;

pub trait Validatable {
    fn validate(&self) -> Option<Vec<Issue>>;
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Issue {
    Warning(String),
    Error(String),
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Issue::Error(message) => write!(f, "(err): {}", message),
            Issue::Warning(message) => write!(f, "(warning): {}", message),
        }
    }
}