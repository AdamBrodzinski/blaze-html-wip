//! Asset tag nom parser
//!
//! Parses the <Script path="assets/foo.js" /> template tag.
//! Parses the <Style path="static/foo.css"/> template tag.
//!
//! If any additional attributes are given, they are passed through 1:1 to the tag:
//!   <Style path="public/asset.css" foo="bar" baz="qux"/>

#![allow(unused)]
use nom::Parser;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::preceded;

use crate::ast::{AssetKind, AssetNode, Attr, TemplateNode};

use super::error::{VResult, make_error};
use super::shared::attrs::{parse_attr, parse_quoted_value};

pub fn parse_script(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<Script").parse(input)?;
    let (input, attrs) = many0(preceded(multispace1, parse_attr)).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = context("closing tag", tag("/>")).parse(input)?;

    let (path, other_attrs) =
        separate_path_attr(attrs, "<Script").map_err(|e| make_error(input, e))?;

    Ok((
        input,
        TemplateNode::Asset(AssetNode {
            kind: AssetKind::Script,
            path,
            attrs: other_attrs,
        }),
    ))
}

pub fn parse_style(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<Style").parse(input)?;
    let (input, attrs) = many0(preceded(multispace1, parse_attr)).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = context("closing tag", tag("/>")).parse(input)?;

    let (path, other_attrs) =
        separate_path_attr(attrs, "<Style").map_err(|e| make_error(input, e))?;

    Ok((
        input,
        TemplateNode::Asset(AssetNode {
            kind: AssetKind::Style,
            path,
            attrs: other_attrs,
        }),
    ))
}

fn separate_path_attr(
    attrs: Vec<(&str, &str, char)>,
    tag_name: &'static str,
) -> Result<(String, Vec<Attr>), String> {
    let mut src_path: Option<&str> = None;
    let mut passthrough_attrs: Vec<Attr> = Vec::new();

    for (key, value, quote) in attrs {
        if key == "path" {
            src_path = Some(value);
            continue;
        }
        passthrough_attrs.push(Attr {
            name: key.to_string(),
            value: value.to_string(),
            quote,
        });
    }

    let src_path = match src_path {
        Some(path) => path.to_string(),
        // custom error trait will convert Err(String) to a nom context error
        None => return Err(format!("path is a required attribute of {tag_name} />")),
    };

    Ok((src_path, passthrough_attrs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    mod script_parser {
        use super::*;

        #[test]
        fn parses_minimal() {
            let template = r#"<Script path="test_files/asset.js"    />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![],
                })
            );
        }

        #[test]
        fn parses_single_quotes() {
            let template = r#"<Script     path='test_files/asset.js'/> other text"#;
            let (remaining, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![],
                })
            );
            assert_eq!(remaining, " other text");
        }

        #[test]
        fn parses_multi_line_spaces() {
            let template = indoc! {r#"
                <Script
                  path='test_files/asset.js'
                /> other text
            "#};
            let (remaining, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![],
                })
            );
            assert_eq!(remaining, " other text\n");
        }

        #[test]
        fn parses_with_attrs() {
            let template = r#"<Script path="test_files/asset.js" foo="bar" baz="qux" />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![
                        Attr {
                            name: "foo".to_string(),
                            value: "bar".to_string(),
                            quote: '"'
                        },
                        Attr {
                            name: "baz".to_string(),
                            value: "qux".to_string(),
                            quote: '"'
                        },
                    ],
                })
            );
        }

        #[test]
        fn preserves_single_quotes() {
            let template = r#"<Script path="test_files/asset.js" foo='bar' baz="qux" />"#;
            let (_remaining, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![
                        Attr {
                            name: "foo".to_string(),
                            value: "bar".to_string(),
                            quote: '\''
                        },
                        Attr {
                            name: "baz".to_string(),
                            value: "qux".to_string(),
                            quote: '"'
                        },
                    ],
                })
            );
        }
    }

    mod style_parser {
        use super::*;

        #[test]
        fn parses_minimal() {
            let template = r#"<Style path="test_files/asset.css"    />"#;
            let (_remaining, node) = parse_style(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Style,
                    path: "test_files/asset.css".to_string(),
                    attrs: vec![],
                })
            );
        }

        #[test]
        fn parses_with_attrs() {
            let template = r#"<Style path="test_files/asset.css" foo="bar" baz="qux"/>"#;
            let (_remaining, node) = parse_style(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Style,
                    path: "test_files/asset.css".to_string(),
                    attrs: vec![
                        Attr {
                            name: "foo".to_string(),
                            value: "bar".to_string(),
                            quote: '"'
                        },
                        Attr {
                            name: "baz".to_string(),
                            value: "qux".to_string(),
                            quote: '"'
                        },
                    ],
                })
            );
        }
    }
}
