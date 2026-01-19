//! Template parsing
//!
//! Parses template strings into an AST for rendering.

pub mod error;
mod shared;
mod tag_asset;
mod tag_component;
mod tag_each;
mod tag_if;
mod text;
mod variables;

use nom::Parser;
use nom::branch::alt;
use nom::multi::many0;

use crate::ast::TemplateNode;
use crate::error::BlazeError;
use crate::error::ParseErrorDetails;

pub fn parse_template_to_ast(page_template: &str) -> crate::error::Result<Vec<TemplateNode>> {
    let (remaining, nodes) = many0(alt((
        tag_each::parse_each,
        tag_if::parse_if,
        tag_asset::parse_script,
        tag_asset::parse_style,
        tag_component::parse_component,
        tag_component::parse_slot,
        variables::parse_escape, // escape and raw syntax must be before parse_variable
        variables::parse_variable_raw,
        variables::parse_variable,
        text::parse_text,
    )))
    .parse(page_template)
    .map_err(|e| BlazeError::from_nom_error(page_template, e))?;

    if !remaining.is_empty() {
        let preview: String = remaining.chars().take(50).collect();
        let position = page_template.len() - remaining.len();

        return Err(BlazeError::Parse(ParseErrorDetails::at_position(
            page_template,
            position,
            format!(
                "Failed to parse template. Unparsed content starting at: {:?}",
                preview
            ),
        )));
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AssetKind, AssetNode};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_ast() {
        let template = indoc! {r#"
            Before
            <Script path="test_files/asset.js" />
            <Style path="test_files/asset.css" />
            foo@@bar.com
            @foo
            After
        "#};
        let ast = parse_template_to_ast(template).unwrap();
        assert_eq!(
            ast,
            [
                TemplateNode::Text("Before\n".into()),
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Script,
                    path: "test_files/asset.js".to_string(),
                    attrs: vec![],
                }),
                TemplateNode::Text("\n".into()),
                TemplateNode::Asset(AssetNode {
                    kind: AssetKind::Style,
                    path: "test_files/asset.css".to_string(),
                    attrs: vec![],
                }),
                TemplateNode::Text("\nfoo".into()),
                TemplateNode::Escaped,
                TemplateNode::Text("bar.com\n".into()),
                TemplateNode::Variable(vec!["foo".into()]),
                TemplateNode::Text("\nAfter\n".into()),
            ]
        );
    }

    #[test]
    fn parse_ast_err() {
        let template = indoc! {r#"
            Before <Script path="test_files/asset.js foo="bar /> After
        "#};
        let result_err = parse_template_to_ast(template).unwrap_err();
        let err_str = result_err.to_string();
        assert!(err_str.contains("<Script"));
        assert!(err_str.contains("Unparsed content"));
    }

    mod parse_asset_err {
        use super::*;
        use indoc::indoc;

        #[test]
        fn missing_attr_quote() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js" foo="bar /> After
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            println!("{}", &err_str);
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("missing closing quote"));
        }

        #[test]
        fn missing_asset_path_attr() {
            let template = indoc! {r#"
               Foo
               <Script foo="bar" />
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            println!("{}", &err_str);
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("path is a required attribute"));
        }

        #[test]
        fn missing_closing_tag() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js" After
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("Unparsed content"));
        }
    }
}
