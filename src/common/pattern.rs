//! This file includes `Pattern`s' data structures and implementations.

use common::validation::{Validatable, Issue};
use regex;
use common::compilation::CompilableTo;

/// The `Pattern` struct represents an uncompiled pattern. Patterns
/// are essentially RegEx searches; given an expression, they _theoretically_
/// will match text. Note that in order for patterns to _actually_ match text,
/// they must first be compiled. (See `CompiledPattern`).
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Pattern {
    /// The pattern to match; either plaintext or a regular expression,
    /// depending on the value of `kind`. Note that RegEx lookbacks are not
    /// supported; all RegEx expressions must search in linear time. See the
    /// Rust `regex` documentation for more information.
    pub content: String,
    
    /// Represents the type of pattern; i.e. RegEx or Raw.
    pub kind: PatternKind,
}

/// `PatternMatch`es are what `CompiledPattern`s output when they encounter
/// text that matches. A `PatternMatch` contains an excerpt of the match.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PatternMatch {
    /// An excerpt of the string being searched that contains the portion
    /// of the text that triggered the match.
    pub excerpt: String,
    /// A tuple of the index of the relevant portion of the `exerpt` that
    /// triggered the match in the form of (start-inclusive, end-exclusive).
    pub relevant: (usize, usize),
}

/// A `CompiledPattern` is a `Pattern` whose RegEx has been compiled or,
/// in the case that the `PatternType` is raw, whose expression has been
/// RegEx escaped and _then_ compiled (as RegEx).
#[derive(Clone)]
pub struct CompiledPattern {
    /// The compiled RegEx of the pattern.
    regex: regex::Regex
}

/// `PatternKind` denotes the type of a pattern. Its two variants, `RegEx`
/// and `Raw`, denote the type of compilation and matching to perform.
/// 
/// * `RegEx` patterns are compiled as RegEx
/// * `Raw` patterns are RegEx escaped and then compiled as RegEx
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum PatternKind {
    /// A RegEx pattern
    RegEx,
    /// A plaintext pattern
    Raw
}

impl Pattern {
    /// Given any pattern, this function returns its expression
    /// as safe-to-compile RegEx. For `Raw` patterns, the expression
    /// is escaped; for `RegEx` patterns, it is cloned. Note that this
    /// function _does not_ validate whether the RegEx is valid; it simply
    /// prepares it for compilation.
    /// 
    /// This is a utility function, and is currently not used during 
    /// compilation.
    pub fn get_as_safe_regex(&self) -> String {
        match self.kind {
            PatternKind::RegEx => self.content.clone(),
            PatternKind::Raw => regex::escape(self.content.as_str())
        }
    }
}

impl CompilableTo<CompiledPattern> for Pattern {
    /// This function compiles the `Pattern` into a `CompiledPattern` by
    /// escaping the RegEx expression as necessary and then compiling it.
    fn compile(&self) -> Result<CompiledPattern, Issue> {
        let regex_pattern = match self.kind {
            PatternKind::Raw => match regex::Regex::new(regex::escape(self.content.as_str()).as_str()) {
                Ok(result) => result,
                Err(_) => return Err(Issue::Error(String::from("escaped regex literal could not compile"))),
            },
            PatternKind::RegEx => match regex::Regex::new(&self.content.as_str()) {
                Ok(result) => result,
                Err(_) => return Err(Issue::Error(String::from("regex could not compile"))),
            }
        };
        Ok(CompiledPattern {
            regex: regex_pattern
        })
    }
}

impl CompiledPattern {
    /// This function performs a 'quick check' for matching on the given string.
    /// It simply returns a boolean value representing whether the string matches
    /// the pattern or not. This function is more performant, but less featureful,
    /// than `full_check`.
    pub fn quick_check(&self, other: &String) -> bool {
        self.regex.is_match(&other)
    }

    /// This function performs a 'full check' on the given text; more specifically,
    /// it determines whether the pattern matches the given text and then, if so,
    /// assembles a `PatternMatch`.
    /// 
    /// Returns `Some(PatternMatch)` if there is a match. Otherwise, the function
    /// returns `None`.
    pub fn full_check(&self, other: &String) -> Option<PatternMatch> {
        match self.regex.find(&other) {
            Some(finding) => {
                Some(PatternMatch {
                    excerpt: other.clone(), // TODO: only include a smaller excerpt, not the whole thing
                    relevant: (finding.start(), finding.end())
                })
            },
            None => None
        }
    }
}

impl Validatable for Pattern {
    /// This function determines whether the `Pattern` is valid.
    /// It performs a compilation check for itself and for its RegEx.
    /// 
    /// Returns `None` if there is no issue; otherwise, `Some(Vec<Issue>)`.
    fn validate(&self) -> Option<Vec<Issue>> {
        match self.compile() {
            Err(issue) => Some(vec![issue]),
            Ok(_) => None
        } // TODO: more expansive (and expensive) checking
    }
}