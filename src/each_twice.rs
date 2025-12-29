#![allow(unused)]

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_until};
use nom::combinator::{rest, verify};
use nom::{IResult, Parser};

type Bytes = [u8];
type Document<'a> = Vec<Node<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum Node<'a> {
    Text(&'a Bytes),
    EachTwo(Vec<Node<'a>>),
}

fn node(input: &Bytes) -> IResult<&Bytes, Node> {
    alt((each_two, text_node)).parse(input)
}

pub fn document(input: &Bytes) -> IResult<&Bytes, Vec<Node>> {
    nom::multi::many0(node).parse(input)
}

fn each_two(input: &Bytes) -> IResult<&Bytes, Node> {
    let (input, _) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, _) = close_each(input)?;
    Ok((input, Node::EachTwo(children)))
}

fn text_node(input: &Bytes) -> IResult<&Bytes, Node> {
    use nom::error::{Error, ErrorKind};
    let len = input.len();
    let mut i = 0;

    while i < len {
        if input[i] == b'<' {
            // Remaining slice starting at '<'
            let rest = &input[i..];

            // Check for opening or closing each-two
            if rest.starts_with(b"<each-two>") || rest.starts_with(b"</each-two>") {
                break;
            }
            // Otherwise, this is some other tag (<foo>, <div>, etc.)
            // Treat '<' as normal text and continue scanning
        }
        i += 1;
    }

    if i == 0 {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::TakeTill1)));
    }

    let (matched, rest) = input.split_at(i);
    Ok((rest, Node::Text(matched)))
}

// fn text_node(input: &Bytes) -> IResult<&Bytes, Node> {
//     // note, this works and test passes, 6 micro seconds
//     let (input, matched) = verify(
//         alt((take_until(b"<each-two>" as &Bytes), take_until(b"</each-two>" as &Bytes), rest)),
//         |s: &Bytes| !s.is_empty(),
//     )
//     .parse(input)?;
//     // note, this works and test passes, 196 nano seconds
//     // let (input, matched) = take_till1(|c: u8| c == b'<').parse(input)?;
//     Ok((input, Node::Text(matched)))
// }

fn open_each(input: &Bytes) -> IResult<&Bytes, &Bytes> {
    tag(b"<each-two>" as &Bytes).parse(input)
}

fn close_each(input: &Bytes) -> IResult<&Bytes, &Bytes> {
    tag(b"</each-two>" as &Bytes).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod text {
        use super::*;
        #[test]
        fn text_and_each_tag() {
            let (remaining, text) = text_node(b"foo <each-two>").unwrap();
            assert_eq!(text, Node::Text(b"foo "));
            assert_eq!(remaining, b"<each-two>");
        }

        #[test]
        fn text_only() {
            let (remaining, text) = text_node(b"foo bar").unwrap();
            assert_eq!(text, Node::Text(b"foo bar"));
            assert_eq!(remaining, b"");
        }
    }
    mod each_tag {
        use super::*;

        #[test]
        fn test_document_input() {
            let (remaining, nodes) = document(b"First <each-two>Inner</each-two> Last").unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn test_nested_document_input() {
            let (remaining, nodes) =
                document(b"BeforeOuter<each-two>BeforeInner<each-two>Inner2</each-two>AfterInner</each-two> AfterOuter")
                    .unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn test_nested_html_input() {
            let (remaining, nodes) = document(
                b"<b>First</b> <each-two>Inner1<each-two>Inner2</each-two></each-two> Last",
            )
            .unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn deep_nesting_with_text() {
            let input = b"<each-two>a<each-two>b<each-two>c</each-two>d</each-two>e</each-two>";
            let (remaining, nodes) = document(input).unwrap();
            assert_eq!(remaining, b"");
        }
    }
}
