#![allow(unused)]
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::rest;
use nom::{IResult, Parser};

// fn node(input: &str) -> IResult<&str, Node> {
// }

// fn document(input: &str) -> IResult<&str, Vec<Node>> {
// }

fn open_each(input: &str) -> IResult<&str, &str> {}

#[cfg(test)]
mod each_twice_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_and_each_tag() {
        assert_eq!("foo", "<each-two>");
    }
}
