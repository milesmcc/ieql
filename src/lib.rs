//! **The reference implementation for IEQL, an open 
//! standard for monitoring Internet content**
//! 
//! This library is the reference implementation for IEQL
//! (Internet Extensible Query Language, pronounced equal).
//! IEQL is an open standard for monitoring and querying 
//! Internet content designed to be fast, efficient, and scalable.

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate regex;
extern crate ron;
extern crate url;
extern crate scraper;

pub mod common;
pub mod query;
pub mod output;
pub mod input;
pub mod scan;

pub use common::pattern::{Pattern, PatternKind};
pub use query::response::{Response, ResponseItem, ResponseKind};
pub use query::scope::{Scope, ScopeContent};
pub use query::threshold::{Threshold, ThresholdConsideration};
pub use query::trigger::Trigger;
pub use query::query::{Query, QueryGroup};
pub use output::output::Output;
pub use scan::scanner::Scanner;
pub use input::document::Document;