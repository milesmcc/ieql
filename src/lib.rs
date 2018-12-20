#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate regex;
extern crate ron;
extern crate url;

pub mod common;
pub mod query;
pub mod output;
pub mod input;
pub mod scan;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
