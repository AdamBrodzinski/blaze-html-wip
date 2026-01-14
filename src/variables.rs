use nom::Parser;
use nom::bytes::complete::tag;
use nom::error::context;
use nom::sequence::preceded;

use crate::ast::TemplateNode;
use crate::parser_error::VResult;

pub fn parse_escape(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = context("escape char", preceded(tag("@"), tag("@"))).parse(input)?;
    Ok((input, TemplateNode::Escaped))
}

pub fn parse_variable(input: &str) -> VResult<'_, TemplateNode> {
    let (input, var_key) = context("variable", preceded(tag("@"), tag("foo"))).parse(input)?;
    Ok((input, TemplateNode::Variable(var_key.into())))
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
            assert_eq!(node, TemplateNode::Variable("foo".into()));
            assert_eq!(remaining, " after");
        }
    }
}
