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
use nom::Parser;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::preceded;

use crate::VResult;
use crate::ast::{AssetKind, AssetNode, TemplateNode};
use crate::parser_error::make_error;
use crate::shared_parsers::attrs::{parse_attr, parse_quoted_value};

impl AssetNode {
    /// Writes the asset node HTML directly into the provided buffer.
    /// This avoids allocations compared to returning a new String.
    pub fn write_html(&self, buf: &mut String) -> Result<(), String> {
        match self.kind {
            AssetKind::Script => {
                buf.push_str(r#"<script src=""#);
                buf.push_str(&self.path);
                write_cache_param(&self.path, buf)?;
                buf.push('"');
                for (k, v) in &self.attrs {
                    buf.push(' ');
                    buf.push_str(k);
                    buf.push_str(r#"=""#);
                    buf.push_str(v);
                    buf.push('"');
                }
                buf.push_str("></script>");
            }
            AssetKind::Style => {
                buf.push_str(r#"<link rel="stylesheet" href=""#);
                buf.push_str(&self.path);
                write_cache_param(&self.path, buf)?;
                buf.push('"');
                for (k, v) in &self.attrs {
                    buf.push(' ');
                    buf.push_str(k);
                    buf.push_str(r#"=""#);
                    buf.push_str(v);
                    buf.push('"');
                }
                buf.push('>');
            }
        }
        Ok(())
    }

}

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
    attrs: Vec<(&str, &str)>,
    tag_name: &'static str,
) -> Result<(String, Vec<(String, String)>), String> {
    let mut src_path: Option<&str> = None;
    let mut passthrough_attrs: Vec<(String, String)> = Vec::new();

    for (key, value) in attrs {
        if key == "path" {
            src_path = Some(value);
            continue;
        }
        passthrough_attrs.push((key.to_string(), value.to_string()));
    }

    let src_path = match src_path {
        Some(path) => path.to_string(),
        // custom error trait will convert Err(String) to a nom context error
        None => return Err(format!("path is a required attribute of {tag_name} />")),
    };

    Ok((src_path, passthrough_attrs))
}

#[cfg(feature = "cache-bust")]
fn write_cache_param(path: &str, buf: &mut String) -> Result<(), String> {
    let hash = hash_file(path).map_err(|e| format!("Failed to hash asset file '{path}': {e}"))?;
    buf.push('?');
    buf.push_str(&hash);
    Ok(())
}

#[cfg(not(feature = "cache-bust"))]
fn write_cache_param(_path: &str, _buf: &mut String) -> Result<(), String> {
    Ok(())
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
                        ("foo".to_string(), "bar".to_string()),
                        ("baz".to_string(), "qux".to_string()),
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
                        ("foo".to_string(), "bar".to_string()),
                        ("baz".to_string(), "qux".to_string()),
                    ],
                })
            );
        }
    }

    #[cfg(feature = "cache-bust")]
    mod write_html_rendering {
        use super::*;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";
        const CSS_HASH: &str = "a0ff2dc6b477abd5ca51c463f720d3ab";

        #[test]
        fn script_renders_with_hash() {
            let asset = AssetNode {
                kind: AssetKind::Script,
                path: "test_files/asset.js".to_string(),
                attrs: vec![],
            };
            let mut buf = String::new();
            asset.write_html(&mut buf).unwrap();
            assert_eq!(
                buf,
                format!(r#"<script src="test_files/asset.js?{JS_HASH}"></script>"#)
            );
        }

        #[test]
        fn script_renders_with_attrs() {
            let asset = AssetNode {
                kind: AssetKind::Script,
                path: "test_files/asset.js".to_string(),
                attrs: vec![
                    ("foo".to_string(), "bar".to_string()),
                    ("baz".to_string(), "qux".to_string()),
                ],
            };
            let mut buf = String::new();
            asset.write_html(&mut buf).unwrap();
            assert_eq!(
                buf,
                format!(
                    r#"<script src="test_files/asset.js?{JS_HASH}" foo="bar" baz="qux"></script>"#
                )
            );
        }

        #[test]
        fn style_renders_with_hash() {
            let asset = AssetNode {
                kind: AssetKind::Style,
                path: "test_files/asset.css".to_string(),
                attrs: vec![],
            };
            let mut buf = String::new();
            asset.write_html(&mut buf).unwrap();
            assert_eq!(
                buf,
                format!(r#"<link rel="stylesheet" href="test_files/asset.css?{CSS_HASH}">"#)
            );
        }

        #[test]
        fn style_renders_with_attrs() {
            let asset = AssetNode {
                kind: AssetKind::Style,
                path: "test_files/asset.css".to_string(),
                attrs: vec![
                    ("foo".to_string(), "bar".to_string()),
                    ("baz".to_string(), "qux".to_string()),
                ],
            };
            let mut buf = String::new();
            asset.write_html(&mut buf).unwrap();
            assert_eq!(
                buf,
                format!(
                    r#"<link rel="stylesheet" href="test_files/asset.css?{CSS_HASH}" foo="bar" baz="qux">"#
                )
            );
        }

        #[test]
        fn returns_error_on_missing_file() {
            let asset = AssetNode {
                kind: AssetKind::Script,
                path: "nonexistent.js".to_string(),
                attrs: vec![],
            };
            let mut buf = String::new();
            let err = asset.write_html(&mut buf).unwrap_err();
            assert!(err.contains("Failed to hash asset file"));
        }
    }
}
