#![allow(unused)]
use nom::IResult;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{pair, preceded};

use nom::Parser;

use crate::ast::TemplateNode;
use crate::shared_parsers::attrs::{parse_attr, parse_quoted_value};

pub fn parse_script(input: &str) -> IResult<&str, TemplateNode> {
    map(
        (
            tag("<Script"),
            many0(preceded(multispace1, parse_attr)),
            pair(multispace0, tag("/>")),
        ),
        |(_, attrs, _)| {
            let (src_path, other_attrs) = separate_path_attr(attrs);
            let text = format!(r#"<script src="{}"{}></script>"#, src_path, other_attrs);
            TemplateNode::Asset(text)
        },
    )
    .parse(input)
}

fn separate_path_attr<'a>(attrs: Vec<(&str, &'a str)>) -> (&'a str, String) {
    let mut src_path: Option<&str> = None;

    let mut passthrough_attrs = String::with_capacity(10 * attrs.len());
    for (key, value) in attrs {
        if key == "path" {
            src_path = Some(value);
            continue;
        }
        passthrough_attrs.push(' ');
        passthrough_attrs.push_str(key);
        passthrough_attrs.push('=');
        passthrough_attrs.push('"');
        passthrough_attrs.push_str(value);
        passthrough_attrs.push('"');
    }

    let src_path = match src_path {
        Some(path) => path,
        None => panic!("'path' is a required field of Script tag"),
    };
    (src_path, passthrough_attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod script {
        use super::*;

        #[test]
        fn minimal() {
            let template = r#"<Script path="static/bar.js"    />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }

        #[test]
        fn single_quotes() {
            let template = r#"<Script     path='static/bar.js'/> other text"#;
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
            assert_eq!(remaining, " other text");
        }

        #[test]
        fn multi_line_spaces() {
            let template = r#"<Script 
              path='static/bar.js' 
            /> other text"#;
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
            assert_eq!(remaining, " other text");
        }

        #[test]
        fn with_attrs() {
            let template = r#"<Script path="static/bar.js" foo="bar" baz="qux" />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js" foo="bar" baz="qux"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }

        // #[test]
        // fn fails_when_path_is_missing() {
        //     let template = r#"<Script foo='bar' />"#;
        //     let result = parse_script(template);
        //     dbg!(&result);
        //     assert!(result.is_err());
        // }
    }
}
