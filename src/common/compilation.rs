use common::validation::Issue;

pub trait CompilableTo<T> {
    fn compile(&self) -> Result<T, Issue>;
}