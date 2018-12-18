#[macro_use]
extern crate serde_derive;

extern crate regex;
extern crate serde;
extern crate serde_json;

pub mod common;
pub mod query;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
