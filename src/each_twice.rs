#![allow(unused)]
type Document = Vec<Node>;

#[derive(Debug, PartialEq, Eq)]
enum Node {
    Text(String),
    EachTwo(Vec<Node>),
}

use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::rest;
use nom::{IResult, Parser};

fn node(input: &str) -> IResult<&str, Node> {
    alt((each_two, text_node)).parse(input)
}

fn document(input: &str) -> IResult<&str, Vec<Node>> {
    nom::multi::many0(node).parse(input)
}

fn each_two(input: &str) -> IResult<&str, Node> {
    let (input, _) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, _) = close_each(input)?;
    Ok((input, Node::EachTwo(children)))
}

fn text_node(input: &str) -> IResult<&str, Node> {
    let (input, _) = alt((take_until("<"), rest)).parse(input)?;
    Ok((input, Node::Text(input.to_owned())))
}

fn open_each(input: &str) -> IResult<&str, &str> {
    tag("<each-two>").parse(input)
}

fn close_each(input: &str) -> IResult<&str, &str> {
    tag("</each-two>").parse(input)
}

#[cfg(test)]
mod each_twice_tests {
    use super::*;
    use serde_json::json;

    mod text {
        use super::*;
        #[test]
        fn text_and_each_tag() {
            let (remaining, text) = text_node("foo <each-two>").unwrap();
            assert_eq!(text, Node::Text("foo ".into()));
            assert_eq!(remaining, "<each-two>");
        }

        #[test]
        fn text_only() {
            let (remaining, text) = text_node("foo bar").unwrap();
            assert_eq!(text, Node::Text("foo bar".into()));
            assert_eq!(remaining, "");
        }
        // #[test]
        // fn text_and_two_each_tags() {
        //     let (remaining, text) = text("foo <each-two> bar <each-two>").unwrap();
        //     assert_eq!(text, "foo ");
        //     assert_eq!(remaining, "<each-two> bar <each-two>");
        // }
    }
    mod each_tag {
        use super::*;
        // fn text_and_each_tag() {
        //     let (remaining, text) = each_two("<each-two>Inner</each-two>").unwrap();
        //     // assert_eq!(text, Node::EachTwo(()));
        //     assert_eq!(remaining, "<each-two>");
        // }
    }
}
