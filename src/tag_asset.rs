#![allow(unused)]
use nom::IResult;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{pair, preceded};

use crate::ast::TemplateNode;
use crate::shared_parsers::attrs::{parse_attr, parse_quoted_value};

use nom::Parser;

pub fn parse_script(input: &str) -> IResult<&str, TemplateNode> {
    map(
        (
            tag("<Script"),
            many0(preceded(multispace1, parse_attr)),
            pair(multispace0, tag("/>")),
        ),
        |(_, attrs, _)| {
            let (src_path, other_attrs) = separate_path_attr(attrs, "<Script src='../path'");
            let text = format!(r#"<script src="{}"{}></script>"#, src_path, other_attrs);
            TemplateNode::Asset(text)
        },
    )
    .parse(input)
}

pub fn parse_style(input: &str) -> IResult<&str, TemplateNode> {
    map(
        (
            tag("<Style"),
            many0(preceded(multispace1, parse_attr)),
            pair(multispace0, tag("/>")),
        ),
        |(_, attrs, _)| {
            let (src_path, other_attrs) = separate_path_attr(attrs, "<Style src='../path'");
            let text = format!(
                r#"<link rel="stylesheet" href="{}"{}>"#,
                src_path, other_attrs
            );
            TemplateNode::Asset(text)
        },
    )
    .parse(input)
}

fn separate_path_attr(attrs: Vec<(&str, &str)>, tag_name: &'static str) -> (String, String) {
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
        None => panic!("'path' is a required field of the {} /> tag", tag_name),
    };

    let src_url = match hash_file(src_path).ok() {
        Some(hash) => format!("{src_path}?{hash}"),
        None => src_path.to_string(),
    };

    (src_url, passthrough_attrs)
}

fn hash_file(path: &str) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(&mut file)?;
    Ok(hasher.finalize().to_hex()[..32].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    mod query_param {
        use super::*;

        #[test]
        fn adds_cache_busting_query_param() {
            let template = r#"<Script path="test_files/asset.js"    />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            let expected_text =
                r#"<script src="test_files/asset.js?a6f2ed7be4c8834436f238d65249b651"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }
    }

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
            let template = indoc! {r#"
                <Script
                  path='static/bar.js'
                /> other 
                 text
            "#};
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
            assert_eq!(remaining, " other \n text\n");
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

    mod style {
        use super::*;

        #[test]
        fn minimal() {
            let template = r#"<Style path="static/bar.css"    />"#;
            let (_remaining, node) = parse_style(template).unwrap();
            let expected_text = r#"<link rel="stylesheet" href="static/bar.css">"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }

        #[test]
        fn with_attrs() {
            let template = r#"<Style path="static/bar.css" foo="bar" baz="qux"/>"#;
            let (_remaining, node) = parse_style(template).unwrap();
            let expected_text =
                r#"<link rel="stylesheet" href="static/bar.css" foo="bar" baz="qux">"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }
    }
}
