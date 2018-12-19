use common::validation::{Validatable, Issue};
use regex;
use common::compilation::CompilableTo;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Pattern {
    pub content: String,
    pub kind: PatternKind,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PatternMatch {
    pub excerpt: String,
    pub relevant: (usize, usize),
}

#[derive(Clone)]
pub struct CompiledPattern {
    regex: regex::Regex
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum PatternKind {
    RegEx,
    Raw
}

impl Pattern {
    // Returns the pattern as regex 
    pub fn get_as_safe_regex(&self) -> String {
        match self.kind {
            PatternKind::RegEx => self.content.clone(),
            PatternKind::Raw => regex::escape(self.content.as_str())
        }
    }
}

impl CompilableTo<CompiledPattern> for Pattern {
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
    pub fn quick_check(&self, other: &String) -> bool {
        self.regex.is_match(&other)
    }

    pub fn full_check(&self, other: &String) -> Option<PatternMatch> {
        match self.regex.find(&other) {
            Some(finding) => {
                Some(PatternMatch {
                    excerpt: other.clone(),
                    relevant: (finding.start(), finding.end())
                })
            },
            None => None
        }
    }
}

impl Validatable for Pattern {
    fn validate(&self) -> Option<Vec<Issue>> {
        unimplemented!();
    }
}