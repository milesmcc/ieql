//! This file provides functionality related to triggers.

use common::pattern::{Pattern, CompiledPattern, PatternMatch};
use common::compilation::CompilableTo;
use common::validation::Issue;

/// Represents a trigger, which is itself mostly a smart 
/// wrapper for JSON expressions.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Trigger {
    /// The pattern that will be evaluated on the text
    pub pattern: Pattern,
    /// The ID of the Trigger (used for `Threshold` evaluation)
    pub id: String,
}

#[derive(Clone)]
pub struct CompiledTrigger {
    pub pattern: CompiledPattern,
    pub id: String,
}

impl CompilableTo<CompiledTrigger> for Trigger {
    fn compile(&self) -> Result<CompiledTrigger, Issue> {
        match self.pattern.compile() {
            Ok(compiled_pattern) => Ok(CompiledTrigger {
                pattern: compiled_pattern,
                id: self.id.clone()
            }),
            Err(issue) => Err(issue)
        }
    }
}

impl CompiledTrigger {
    /// Checks if the `Trigger` matches the given string
    /// without extracting any type of excerpt.
    /// 
    /// This is typically much faster than performing a
    /// `full_check()`.
    pub fn quick_check(&self, other: &String) -> bool {
        self.pattern.quick_check(other)
    }

    /// Checks if the `Trigger` matches the given string
    /// and extracts an excerpt.
    /// 
    /// This is typically slower than `quick_check()`, and
    /// in most scenarios it makes sense to run `quick_check()`
    /// first before running this function.
    pub fn full_check(&self, other: &String) -> Option<PatternMatch> {
        self.pattern.full_check(other)
    }
}
