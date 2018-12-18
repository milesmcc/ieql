use common::pattern::{CompiledPattern, Pattern};
use common::compilation::CompilableTo;
use common::validation::Issue;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Scope {
    pub pattern: Pattern,
    pub content: ScopeContent
}

#[derive(Clone)]
pub struct CompiledScope {
    pattern: CompiledPattern,
    content: ScopeContent
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ScopeContent {
    Raw,
    Text
}

impl CompilableTo<CompiledScope> for Scope {
    fn compile(&self) -> Result<CompiledScope, Issue> {
        match self.pattern.compile() {
            Ok(compiled_pattern) => Ok(CompiledScope {
                pattern: compiled_pattern,
                content: self.content,
            }),
            Err(issue) => Err(issue)
        }
    }
}