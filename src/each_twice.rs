#![allow(unused)]
type Document = Vec<Node>;

enum Node {
    Text(String),
    EachTwo(Vec<Node>),
}

use nom::bytes::complete::take_until;
use nom::{IResult, Parser};

fn text(input: &str) -> IResult<&str, &str> {
    take_until("<").parse(input)
}

#[cfg(test)]
mod each_twice_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn basic_array_of_objects() {
        let (remaining, text) = text("foo <div>").unwrap();
        assert_eq!(text, "foo ");
        assert_eq!(remaining, "<div>");
    }
}

