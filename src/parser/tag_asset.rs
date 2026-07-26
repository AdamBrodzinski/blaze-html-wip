//! Asset tag nom parser
//!
//! Parses asset template tags:
//! - <Script path="assets/foo.js" />
//! - <Style path="static/foo.css" />
//! - <Preload path="assets/logo.webp" as="image" />
//! - <Icon path="assets/favicon.png" sizes="32x32" />
//! - <Image path="assets/photo.webp" alt="Photo" />
//!
//! The path attribute is required. Any additional attributes are passed through 1:1
//! to the rendered HTML tag.

use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::verify;
use nom::error::context;
use nom::multi::many0;
use nom::sequence::preceded;

use crate::ast::{AssetKind, AssetNode, Attr, TemplateNode};

use super::error::{VResult, make_error};
use super::shared::attrs::{parse_attr, parse_attr_name};

pub fn parse_icon(input: &str) -> VResult<'_, TemplateNode> {
    parse_asset(input, "<Icon", AssetKind::Icon)
}

pub fn parse_image(input: &str) -> VResult<'_, TemplateNode> {
    parse_asset(input, "<Image", AssetKind::Image)
}

pub fn parse_preload(input: &str) -> VResult<'_, TemplateNode> {
    parse_asset(input, "<Preload", AssetKind::Preload)
}

pub fn parse_script(input: &str) -> VResult<'_, TemplateNode> {
    parse_asset(input, "<Script", AssetKind::Script)
}

pub fn parse_style(input: &str) -> VResult<'_, TemplateNode> {
    parse_asset(input, "<Style", AssetKind::Style)
}

fn parse_asset<'a>(
    input: &'a str,
    tag_name: &'static str,
    kind: AssetKind,
) -> VResult<'a, TemplateNode> {
    let (input, _) = tag(tag_name).parse(input)?;
    let (input, attrs) =
        many0(preceded(multispace1, alt((parse_attr, parse_boolean_attr)))).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = context("closing tag", tag("/>")).parse(input)?;

    let (path, other_attrs) =
        separate_path_attr(attrs, tag_name).map_err(|e| make_error(input, e))?;

    Ok((
        input,
        TemplateNode::Asset(AssetNode {
            kind,
            path,
            attrs: other_attrs,
        }),
    ))
}

fn parse_boolean_attr(input: &str) -> VResult<'_, (&str, &str, char)> {
    verify(parse_attr_name, |name: &str| name != "path")
        .map(|name| (name, name, '"'))
        .parse(input)
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
        Some("") => return Err(format!("path attribute of {tag_name} /> cannot be empty")),
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

    #[test]
    fn parses_minimal_icon_and_preload() {
        let (_, icon) = parse_icon(r#"<Icon path="assets/favicon.png" />"#).unwrap();
        let (_, preload) = parse_preload(r#"<Preload path="assets/logo.webp" />"#).unwrap();

        assert_eq!(
            icon,
            TemplateNode::Asset(AssetNode {
                kind: AssetKind::Icon,
                path: "assets/favicon.png".to_string(),
                attrs: vec![],
            })
        );
        assert_eq!(
            preload,
            TemplateNode::Asset(AssetNode {
                kind: AssetKind::Preload,
                path: "assets/logo.webp".to_string(),
                attrs: vec![],
            })
        );
    }

    mod image_parser {
        use super::*;

        #[test]
        fn parses_with_passthrough_attrs() {
            let template =
                r#"<Image path='assets/photo.webp' alt="Photo" width='640' loading="lazy" /> rest"#;
            let (remaining, node) = parse_image(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Image,
                    path: "assets/photo.webp".to_string(),
                    attrs: vec![
                        Attr {
                            name: "alt".to_string(),
                            value: "Photo".to_string(),
                            quote: '"',
                        },
                        Attr {
                            name: "width".to_string(),
                            value: "640".to_string(),
                            quote: '\'',
                        },
                        Attr {
                            name: "loading".to_string(),
                            value: "lazy".to_string(),
                            quote: '"',
                        },
                    ],
                })
            );
            assert_eq!(remaining, " rest");
        }

        #[test]
        fn requires_path() {
            assert!(parse_image(r#"<Image alt="Photo" />"#).is_err());
        }

        #[test]
        fn rejects_empty_path() {
            let err = parse_image(r#"<Image path="" alt="Photo" />"#).unwrap_err();
            assert!(format!("{err:?}").contains("path attribute of <Image /> cannot be empty"));
        }
    }

    mod icon_parser {
        use super::*;

        #[test]
        fn parses_with_passthrough_attrs() {
            let template =
                r#"<Icon path='assets/favicon.png' sizes='32x32' type="image/png" /> rest"#;
            let (remaining, node) = parse_icon(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Icon,
                    path: "assets/favicon.png".to_string(),
                    attrs: vec![
                        Attr {
                            name: "sizes".to_string(),
                            value: "32x32".to_string(),
                            quote: '\'',
                        },
                        Attr {
                            name: "type".to_string(),
                            value: "image/png".to_string(),
                            quote: '"',
                        },
                    ],
                })
            );
            assert_eq!(remaining, " rest");
        }

        #[test]
        fn requires_path() {
            assert!(parse_icon(r#"<Icon sizes="32x32" />"#).is_err());
        }
    }

    mod preload_parser {
        use super::*;

        #[test]
        fn parses_multiline_with_passthrough_attrs() {
            let template = indoc! {r#"
                <Preload
                  path="assets/logo.webp"
                  as='image'
                  type="image/webp"
                  crossorigin="anonymous"
                  fetchpriority="high"
                  media="(min-width: 800px)"
                /> rest
            "#};
            let (remaining, node) = parse_preload(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Preload,
                    path: "assets/logo.webp".to_string(),
                    attrs: vec![
                        Attr {
                            name: "as".to_string(),
                            value: "image".to_string(),
                            quote: '\'',
                        },
                        Attr {
                            name: "type".to_string(),
                            value: "image/webp".to_string(),
                            quote: '"',
                        },
                        Attr {
                            name: "crossorigin".to_string(),
                            value: "anonymous".to_string(),
                            quote: '"',
                        },
                        Attr {
                            name: "fetchpriority".to_string(),
                            value: "high".to_string(),
                            quote: '"',
                        },
                        Attr {
                            name: "media".to_string(),
                            value: "(min-width: 800px)".to_string(),
                            quote: '"',
                        },
                    ],
                })
            );
            assert_eq!(remaining, " rest\n");
        }

        #[test]
        fn requires_path() {
            assert!(parse_preload(r#"<Preload as="image" />"#).is_err());
        }
    }

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
        fn parses_boolean_attrs() {
            let template = r#"<Script path="test_files/asset.js" defer async />"#;
            let (_, node) = parse_script(template).unwrap();
            assert_eq!(
                node,
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![
                        Attr {
                            name: "defer".to_string(),
                            value: "defer".to_string(),
                            quote: '"',
                        },
                        Attr {
                            name: "async".to_string(),
                            value: "async".to_string(),
                            quote: '"',
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
