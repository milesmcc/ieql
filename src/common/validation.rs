//! This file provides validation functionality for use with
//! `ieql validate` (among other functions).

use std::fmt;

/// This trait provides types with the `validate` function. It is useful
/// for types whose data structures can have many different states, only _some_
/// of which are valid.
pub trait Validatable {
    /// This function determines whether `self` is valid. When it _is_ valid,
    /// this function will return `None`. Otherwise, if there are issues,
    /// it will return `Some(Vec<Issue>)`.
    fn validate(&self) -> Option<Vec<Issue>>;
}

/// There are two types of issues: serious and non-serious. For recoverable
/// problems that still may be of note to the user, use `Warning`. For more severe
/// issues, use `Error`. The associated enum `String` should be a human-readable
/// description of the issue, suitable, for error logs.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Issue {
    /// For recoverable issues, but still worth alerting the user
    Warning(String),
    /// For severe or otherwise unrecoverable issues
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