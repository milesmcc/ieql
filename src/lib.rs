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

pub use query::query::{CompiledQuery, Query};
pub use output::output::Output;
pub use scan::scanner::Scanner;
pub use input::document::Document;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
