#![allow(unused)]

type Document = Vec<Node>;

#[derive(Debug, PartialEq, Eq)]
pub enum Node {
    Text(String),
    EachTwo(Vec<Node>),
}

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_until};
use nom::combinator::{rest, verify};
use nom::{IResult, Parser};

fn node(input: &str) -> IResult<&str, Node> {
    alt((each_two, text_node)).parse(input)
}

pub fn document(input: &str) -> IResult<&str, Vec<Node>> {
    nom::multi::many0(node).parse(input)
}

fn each_two(input: &str) -> IResult<&str, Node> {
    let (input, _) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, matched) = close_each(input)?;
    Ok((input, Node::EachTwo(children)))
}

fn text_node(input: &str) -> IResult<&str, Node> {
    // note, this works and test passes, 6 micro seconds
    let (input, matched) = verify(
        alt((take_until("<each-two>"), take_until("</each-two>"), rest)),
        |s: &str| !s.is_empty(),
    )
    .parse(input)?;
    // note, this works and test passes, 196 nano seconds
    // let (input, matched) = take_till1(|c| c == '<').parse(input)?;
    Ok((input, Node::Text(matched.to_owned())))
}

fn open_each(input: &str) -> IResult<&str, &str> {
    tag("<each-two>").parse(input)
}

fn close_each(input: &str) -> IResult<&str, &str> {
    tag("</each-two>").parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod text {
        use super::*;
        // #[test]
        // fn text_and_each_tag() {
        //     let (remaining, text) = text_node("foo <each-two>").unwrap();
        //     assert_eq!(text, Node::Text("foo ".into()));
        //     assert_eq!(remaining, "<each-two>");
        // }
        //
        // #[test]
        // fn text_only() {
        //     let (remaining, text) = text_node("foo bar").unwrap();
        //     assert_eq!(text, Node::Text("foo bar".into()));
        //     assert_eq!(remaining, "");
        // }
    }
    mod each_tag {
        use super::*;

        #[test]
        fn test_document_input() {
            let (remaining, nodes) = document("First <each-two>Inner</each-two> Last").unwrap();
            // dbg!(remaining);
            // dbg!(nodes);
            assert_eq!(remaining, "");
            // assert_eq!(nodes[0], Node::Text("First ".into()));
            // assert_eq!(&nodes[1], Node::Text("First ".into()));
            // assert_eq!("", Node::EachTwo(()));
        }

        // #[test]
        // fn text_and_two_each_tags() {
        //     let (remaining, text) = text("foo <each-two> bar <each-two>").unwrap();
        //     assert_eq!(text, "foo ");
        //     assert_eq!(remaining, "<each-two> bar <each-two>");
        // }
    }
}
