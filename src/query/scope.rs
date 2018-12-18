use common::pattern::{CompiledPattern, Pattern};
use common::compilation::CompilableTo;
use common::validation::Issue;

#[derive(Clone, Serialize, Deserialize)]
pub struct Scope {
    documents: Pattern,
    content: ScopeContent
}

#[derive(Clone)]
pub struct CompiledScope {
    documents: CompiledPattern,
    content: ScopeContent
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum ScopeContent {
    Raw,
    Text
}

impl CompilableTo<CompiledScope> for Scope {
    fn compile(&self) -> Result<CompiledScope, Issue> {
        match self.documents.compile() {
            Ok(compiled_pattern) => Ok(CompiledScope {
                documents: compiled_pattern,
                content: self.content,
            }),
            Err(issue) => Err(issue)
        }
    }
}