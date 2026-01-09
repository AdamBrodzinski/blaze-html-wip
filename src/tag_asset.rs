//! Asset tag nom parser
//!
//! Parses the <Script path="assets/foo.js" /> template tag and transforms it
//! into a <script src="assets/foo.js?1a2b3c"></script> tag that includes a cache busting
//! query param. The <Style path="static/foo.css"/> does the same except it
//! renders a <link> tag
//!
//! If any additional attributes are given, they are passed through 1:1 to the tag:
//!   <Style path="public/asset.css" foo="bar" baz="qux"/>
//!   <link rel="stylesheet" href="test_files/asset.css?1a2b3c" foo="bar" baz="qux">
//!

#![allow(unused)]
use nom::IResult;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::map_res;
use nom::multi::many0;
use nom::sequence::{pair, preceded};

use crate::ast::TemplateNode;
use crate::shared_parsers::attrs::{parse_attr, parse_quoted_value};

use nom::Parser;

pub fn parse_script(input: &str) -> IResult<&str, TemplateNode> {
    map_res(
        (
            tag("<Script"),
            many0(preceded(multispace1, parse_attr)),
            pair(multispace0, tag("/>")),
        ),
        |(_, attrs, _)| {
            let (src_path, other_attrs) = separate_path_attr(attrs, "<Script src='../path'")?;
            let text = format!(r#"<script src="{}"{}></script>"#, src_path, other_attrs);
            Ok::<_, std::io::Error>(TemplateNode::Asset(text))
        },
    )
    .parse(input)
}

pub fn parse_style(input: &str) -> IResult<&str, TemplateNode> {
    map_res(
        (
            tag("<Style"),
            many0(preceded(multispace1, parse_attr)),
            pair(multispace0, tag("/>")),
        ),
        |(_, attrs, _)| {
            let (src_path, other_attrs) = separate_path_attr(attrs, "<Style src='../path'")?;
            let text = format!(
                r#"<link rel="stylesheet" href="{}"{}>"#,
                src_path, other_attrs
            );
            Ok::<_, std::io::Error>(TemplateNode::Asset(text))
        },
    )
    .parse(input)
}

fn separate_path_attr(
    attrs: Vec<(&str, &str)>,
    tag_name: &'static str,
) -> Result<(String, String), std::io::Error> {
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

    let cache_param = get_cache_param(src_path)?;
    Ok((format!("{src_path}{cache_param}"), passthrough_attrs))
}

#[cfg(feature = "cache-bust")]
fn get_cache_param(path: &str) -> Result<String, std::io::Error> {
    let hash = hash_file(path)?;
    Ok(format!("?{hash}"))
}

#[cfg(not(feature = "cache-bust"))]
fn get_cache_param(_path: &str) -> Result<String, std::io::Error> {
    Ok(String::new())
}

#[cfg(feature = "cache-bust")]
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

    #[cfg(feature = "cache-bust")]
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

    #[cfg(feature = "cache-bust")]
    mod script {
        use super::*;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";

        #[test]
        fn minimal() {
            let template = r#"<Script path="test_files/asset.js"    />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            let expected_text = format!(r#"<script src="test_files/asset.js?{JS_HASH}"></script>"#);
            assert_eq!(node, TemplateNode::Asset(expected_text));
        }

        #[test]
        fn single_quotes() {
            let template = r#"<Script     path='test_files/asset.js'/> other text"#;
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = format!(r#"<script src="test_files/asset.js?{JS_HASH}"></script>"#);
            assert_eq!(node, TemplateNode::Asset(expected_text));
            assert_eq!(remaining, " other text");
        }

        #[test]
        fn multi_line_spaces() {
            let template = indoc! {r#"
                <Script
                  path='test_files/asset.js'
                /> other text
            "#};
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = format!(r#"<script src="test_files/asset.js?{JS_HASH}"></script>"#);
            assert_eq!(node, TemplateNode::Asset(expected_text));
            assert_eq!(remaining, " other text\n");
        }

        #[test]
        fn with_attrs() {
            let template = r#"<Script path="test_files/asset.js" foo="bar" baz="qux" />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            let expected_text = format!(
                r#"<script src="test_files/asset.js?{JS_HASH}" foo="bar" baz="qux"></script>"#
            );
            assert_eq!(node, TemplateNode::Asset(expected_text));
        }

        #[test]
        fn fails_on_missing_file() {
            let template = r#"<Script path="nonexistent.js" />"#;
            let result = parse_script(template);
            assert!(result.is_err());
        }
    }

    #[cfg(feature = "cache-bust")]
    mod style {
        use super::*;

        const CSS_HASH: &str = "a0ff2dc6b477abd5ca51c463f720d3ab";

        #[test]
        fn minimal() {
            let template = r#"<Style path="test_files/asset.css"    />"#;
            let (_remaining, node) = parse_style(template).unwrap();
            let expected_text =
                format!(r#"<link rel="stylesheet" href="test_files/asset.css?{CSS_HASH}">"#);
            assert_eq!(node, TemplateNode::Asset(expected_text));
        }

        #[test]
        fn with_attrs() {
            let template = r#"<Style path="test_files/asset.css" foo="bar" baz="qux"/>"#;
            let (_remaining, node) = parse_style(template).unwrap();
            let expected_text = format!(
                r#"<link rel="stylesheet" href="test_files/asset.css?{CSS_HASH}" foo="bar" baz="qux">"#
            );
            assert_eq!(node, TemplateNode::Asset(expected_text));
        }

        #[test]
        fn fails_on_missing_file() {
            let template = r#"<Style path="nonexistent.css" />"#;
            let result = parse_style(template);
            assert!(result.is_err());
        }
    }
}
