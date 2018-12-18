#[macro_use]
extern crate serde_derive;
extern crate serde;

extern crate regex;
extern crate ron;

pub mod common;
pub mod query;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
