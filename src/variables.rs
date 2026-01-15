use nom::Parser;
use nom::bytes::complete::{tag, take_while1};
use nom::error::context;
use nom::sequence::preceded;

use crate::ast::TemplateNode;
use crate::parser_error::VResult;

pub fn parse_escape(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = context("escape char", preceded(tag("@"), tag("@"))).parse(input)?;
    Ok((input, TemplateNode::Escaped))
}

pub fn parse_variable(input: &str) -> VResult<'_, TemplateNode> {
    let (input, var_key) = context(
        "variable",
        preceded(
            tag("@"),
            take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_'),
        ),
    )
    .parse(input)?;
    let segments: Vec<String> = var_key.split('.').map(String::from).collect();
    Ok((input, TemplateNode::Variable(segments)))
}

/// Parses raw/unescaped variables with @! prefix (e.g., @!foo, @!user.html_content)
pub fn parse_variable_raw(input: &str) -> VResult<'_, TemplateNode> {
    let (input, var_key) = context(
        "raw variable",
        preceded(
            tag("@!"),
            take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_'),
        ),
    )
    .parse(input)?;
    let segments: Vec<String> = var_key.split('.').map(String::from).collect();
    Ok((input, TemplateNode::VariableRaw(segments)))
}

#[cfg(test)]
mod tests {
    use super::*;
    // use indoc::indoc;

    mod escape {
        use super::*;

        #[test]
        fn parses_email() {
            let template = "@@gmail.com";
            let (remaining, node) = parse_escape(template).unwrap();
            assert_eq!(node, TemplateNode::Escaped);
            assert_eq!(remaining, "gmail.com");
        }

        #[test]
        fn parses_media_query() {
            let template = "@@media print {";
            let (remaining, node) = parse_escape(template).unwrap();
            assert_eq!(node, TemplateNode::Escaped);
            assert_eq!(remaining, "media print {");
        }
    }

    mod variable {
        use super::*;

        #[test]
        fn parses_simple_variable() {
            let template = "@foo after";
            let (remaining, node) = parse_variable(template).unwrap();
            assert_eq!(node, TemplateNode::Variable(vec!["foo".into()]));
            assert_eq!(remaining, " after");
        }

        #[test]
        fn parses_variable_with_number() {
            let template = "@foo1 after";
            let (remaining, node) = parse_variable(template).unwrap();
            assert_eq!(node, TemplateNode::Variable(vec!["foo1".into()]));
            assert_eq!(remaining, " after");
        }

        #[test]
        fn parses_variable_with_underscore() {
            let template = "@first_name2 after";
            let (remaining, node) = parse_variable(template).unwrap();
            assert_eq!(node, TemplateNode::Variable(vec!["first_name2".into()]));
            assert_eq!(remaining, " after");
        }

        #[test]
        fn parses_nested_variable() {
            let template = "@person.first_name2 after";
            let (remaining, node) = parse_variable(template).unwrap();
            assert_eq!(node, TemplateNode::Variable(vec!["person".into(), "first_name2".into()]));
            assert_eq!(remaining, " after");
        }
    }

    mod variable_raw {
        use super::*;

        #[test]
        fn parses_simple_raw_variable() {
            let template = "@!foo after";
            let (remaining, node) = parse_variable_raw(template).unwrap();
            assert_eq!(node, TemplateNode::VariableRaw(vec!["foo".into()]));
            assert_eq!(remaining, " after");
        }

        #[test]
        fn parses_nested_raw_variable() {
            let template = "@!user.html_content after";
            let (remaining, node) = parse_variable_raw(template).unwrap();
            assert_eq!(node, TemplateNode::VariableRaw(vec!["user".into(), "html_content".into()]));
            assert_eq!(remaining, " after");
        }
    }
}
