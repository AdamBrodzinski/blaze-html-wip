//! Include tag nom parser
//!
//! Parses the <Include path="partials/foo.css" /> template tag.
//!
//! Unlike <Script>/<Style>, the include has no passthrough attributes: the only
//! recognized attribute is `path`. At render time the file is read relative to the
//! template root and its raw contents are spliced into the output verbatim.

use nom::Parser;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::preceded;

use crate::ast::TemplateNode;

use super::error::{VResult, make_error};
use super::shared::attrs::parse_attr;

pub fn parse_include(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<Include").parse(input)?;
    let (input, attrs) = many0(preceded(multispace1, parse_attr)).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = context("closing tag", tag("/>")).parse(input)?;

    let path = extract_path(attrs, "<Include").map_err(|e| make_error(input, e))?;

    Ok((input, TemplateNode::Include(path)))
}

fn extract_path(attrs: Vec<(&str, &str, char)>, tag_name: &str) -> Result<String, String> {
    let mut path: Option<String> = None;

    for (key, value, _quote) in attrs {
        match key {
            "path" => path = Some(value.to_string()),
            other => {
                return Err(format!(
                    "unknown attribute {other:?} on {tag_name} (only `path` is supported)"
                ));
            }
        }
    }

    // custom error trait converts Err(String) to a nom context error
    match path {
        Some(path) if path.is_empty() => {
            Err(format!("path attribute of {tag_name} /> cannot be empty"))
        }
        Some(path) => Ok(path),
        None => Err(format!("path is a required attribute of {tag_name} />")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parses_minimal() {
        let template = r#"<Include path="partials/view.css"/>"#;
        let (remaining, node) = parse_include(template).unwrap();
        assert_eq!(node, TemplateNode::Include("partials/view.css".to_string()));
        assert_eq!(remaining, "");
    }

    #[test]
    fn parses_single_quotes() {
        let template = r#"<Include    path='partials/view.css' /> other text"#;
        let (remaining, node) = parse_include(template).unwrap();
        assert_eq!(node, TemplateNode::Include("partials/view.css".to_string()));
        assert_eq!(remaining, " other text");
    }

    #[test]
    fn parses_multi_line_spaces() {
        let template = indoc! {r#"
            <Include
              path='partials/view.css'
            /> other text
        "#};
        let (remaining, node) = parse_include(template).unwrap();
        assert_eq!(node, TemplateNode::Include("partials/view.css".to_string()));
        assert_eq!(remaining, " other text\n");
    }

    #[test]
    fn missing_path_errors() {
        let template = r#"<Include />"#;
        let err = parse_include(template).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("path is a required attribute"), "got: {msg}");
    }

    #[test]
    fn empty_path_errors() {
        let template = r#"<Include path=""/>"#;
        let err = parse_include(template).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("path attribute of <Include /> cannot be empty"),
            "got: {msg}"
        );
    }

    #[test]
    fn unknown_attr_errors() {
        let template = r#"<Include path="a.css" foo="bar"/>"#;
        let err = parse_include(template).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("unknown attribute"), "got: {msg}");
    }
}
