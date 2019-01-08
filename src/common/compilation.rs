//! This file provides the `CompilableTo<T>` trait.

use common::validation::Issue;

/// The `CompilableTo<T>` trait specifies the compilation interface for each
/// of IEQL's compilable internals, including `Document`s and `Query` groups,
/// among others. 
/// 
/// The fundamental idea of compilation in the Rust IEQL implementation is that
/// every component of a scan has a certain amount of 'overhead'—that is, logic
/// that must be repeated. For example, in `Pattern`s, the RegEx must be composed
/// by the RegEx library into something that can be used to scan text. This is a
/// relatively expensive operation, and therefore it makes no sense to perform it
/// multiple times.
/// 
/// In essence, compilation performs the repeated expensive logic for various
/// IEQL components ahead of time to save time later.
pub trait CompilableTo<T> {
    /// Returns a compiled version of `self`, _without_ using self.
    /// Importantly, compiling a component will perform a deep clone on that
    /// component. **Remember: compilation is expensive!**
    fn compile(&self) -> Result<T, Issue>;
}