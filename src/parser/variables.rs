use nom::Parser;
use nom::bytes::complete::{tag, take_while1};
use nom::combinator::recognize;
use nom::error::context;
use nom::multi::many0;
use nom::sequence::preceded;

use crate::ast::TemplateNode;

use super::error::VResult;

pub(super) fn parse_escape(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = context("escape char", preceded(tag("@"), tag("@"))).parse(input)?;
    Ok((input, TemplateNode::Escaped))
}

pub(super) fn parse_variable(input: &str) -> VResult<'_, TemplateNode> {
    let (input, var_key) =
        context("variable", preceded(tag("@"), parse_variable_path)).parse(input)?;
    let segments: Vec<String> = var_key.split('.').map(String::from).collect();
    Ok((input, TemplateNode::Variable(segments)))
}

/// Parses raw/unescaped variables with @! prefix (e.g., @!foo, @!user.html_content)
pub(super) fn parse_variable_raw(input: &str) -> VResult<'_, TemplateNode> {
    let (input, var_key) =
        context("raw variable", preceded(tag("@!"), parse_variable_path)).parse(input)?;
    let segments: Vec<String> = var_key.split('.').map(String::from).collect();
    Ok((input, TemplateNode::VariableRaw(segments)))
}

fn parse_variable_path(input: &str) -> VResult<'_, &str> {
    recognize((
        take_while1(is_variable_segment_char),
        many0(preceded(tag("."), take_while1(is_variable_segment_char))),
    ))
    .parse(input)
}

fn is_variable_segment_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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
            assert_eq!(
                node,
                TemplateNode::Variable(vec!["person".into(), "first_name2".into()])
            );
            assert_eq!(remaining, " after");
        }

        #[test]
        fn leaves_sentence_period_after_variable() {
            let (remaining, node) = parse_variable("@person.name.").unwrap();
            assert_eq!(
                node,
                TemplateNode::Variable(vec!["person".into(), "name".into()])
            );
            assert_eq!(remaining, ".");
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
            assert_eq!(
                node,
                TemplateNode::VariableRaw(vec!["user".into(), "html_content".into()])
            );
            assert_eq!(remaining, " after");
        }

        #[test]
        fn leaves_sentence_period_after_raw_variable() {
            let (remaining, node) = parse_variable_raw("@!article.html.").unwrap();
            assert_eq!(
                node,
                TemplateNode::VariableRaw(vec!["article".into(), "html".into()])
            );
            assert_eq!(remaining, ".");
        }
    }
}
