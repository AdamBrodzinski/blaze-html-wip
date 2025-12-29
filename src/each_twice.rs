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

fn process_each(input: &str) -> Result<String, String> {
    let mut buffer = String::new();
    let (_, nodes) = document(input.as_bytes()).map_err(|e| e.to_string())?;

    for node in nodes {
        dbg!(node);
        match node {
            Text(txt) => {
                buffer.push_str("Text");
            }
            EachTwo(txt) => {
                buffer.push_str("Each");
            }
        }
    }

    Ok(String::from(""))
}

fn node(input: &'_ Bytes) -> IResult<&'_ Bytes, Node<'_>> {
    alt((each_two, text_node)).parse(input)
}

pub fn document(input: &'_ Bytes) -> IResult<&'_ Bytes, Vec<Node<'_>>> {
    nom::multi::many0(node).parse(input)
}

fn each_two(input: &Bytes) -> IResult<&Bytes, Node<'_>> {
    let (input, _) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, _) = close_each(input)?;
    Ok((input, Node::EachTwo(children)))
}

fn text_node(input: &Bytes) -> IResult<&Bytes, Node<'_>> {
    use nom::error::{Error, ErrorKind};
    let len = input.len();
    let mut i = 0;
    // manually parse text
    while i < len {
        if input[i] == b'<' {
            let rest = &input[i..];
            if rest.starts_with(b"<Each>") || rest.starts_with(b"</Each>") {
                break;
            }
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
//         alt((
//             take_until(b"<Each>" as &Bytes),
//             take_until(b"</Each>" as &Bytes),
//             rest,
//         )),
//         |s: &Bytes| !s.is_empty(),
//     )
//     .parse(input)?;
//     // note, this works and test passes, 196 nano seconds
//     // let (input, matched) = take_till1(|c: u8| c == b'<').parse(input)?;
//     Ok((input, Node::Text(matched)))
// }

fn open_each(input: &Bytes) -> IResult<&Bytes, &Bytes> {
    tag(b"<Each>" as &Bytes).parse(input)
}

fn close_each(input: &Bytes) -> IResult<&Bytes, &Bytes> {
    tag(b"</Each>" as &Bytes).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod process {
        use super::*;

        #[test]
        fn test_process_each() {
            let result = process_each("First <Each>Inner</Each> Last").unwrap();
            assert_eq!(result, "foo");
        }
    }

    mod text {
        use super::*;
        #[test]
        fn text_and_each_tag() {
            let (remaining, text) = text_node(b"foo <Each>").unwrap();
            assert_eq!(text, Node::Text(b"foo "));
            assert_eq!(remaining, b"<Each>");
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
        fn test_process_each() {
            let result = process_each("First <Each>Inner</Each> Last").unwrap();
            assert_eq!(result, "foo");
        }

        #[test]
        fn test_document_input() {
            let (remaining, nodes) = document(b"First <Each>Inner</Each> Last").unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn test_nested_document_input() {
            let (remaining, nodes) = document(
                b"BeforeOuter<Each>BeforeInner<Each>Inner2</Each>AfterInner</Each> AfterOuter",
            )
            .unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn test_nested_html_input() {
            let (remaining, nodes) =
                document(b"<b>First</b> <Each>Inner1<Each>Inner2</Each></Each> Last").unwrap();
            assert_eq!(remaining, b"");
        }

        #[test]
        fn deep_nesting_with_text() {
            let input = b"<Each>a<Each>b<Each>c</Each>d</Each>e</Each>";
            let (remaining, nodes) = document(input).unwrap();
            assert_eq!(remaining, b"");
        }
    }
}
