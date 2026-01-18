use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::cut;
use nom::error::context;

use crate::ast::{IfNode, TemplateNode};

use super::error::{VResult, make_error};
use super::shared::attrs::parse_quoted_value;

pub fn parse_if(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<If").parse(input)?;
    let (input, _) = multispace1.parse(input)?;

    // parse either true="@path" or false="@path"
    let (input, (negate, condition_path)) = parse_condition_attr(input)?;

    let (input, _) = multispace0.parse(input)?;

    let (input, _) = context("if opening tag >", cut(tag(">"))).parse(input)?;

    // walk forward to the matching </If>, accounting for nested If tags
    let (remaining, inner_content) =
        parse_until_closing_if(input).map_err(|e| make_error(input, e))?;

    // parse inner content recursively
    let children = super::parse_template_to_ast(inner_content)
        .map_err(|e| make_error(inner_content, format!("Error parsing If body: {}", e)))?;

    Ok((
        remaining,
        TemplateNode::If(IfNode {
            condition_path,
            negate,
            children,
        }),
    ))
}

fn parse_condition_attr(input: &str) -> VResult<'_, (bool, Vec<String>)> {
    // Try true="@path" first, then false="@path"
    alt((parse_true_attr, parse_false_attr)).parse(input)
}

fn parse_true_attr(input: &str) -> VResult<'_, (bool, Vec<String>)> {
    let (input, _) = context("true attribute", tag("true")).parse(input)?;
    let (input, _) = cut(tag("=")).parse(input)?;
    let (input, (_, value)) = cut(context("true value", parse_quoted_value)).parse(input)?;

    let path = parse_condition_path(value).map_err(|e| make_error(input, e))?;

    Ok((input, (false, path))) // negate=false for true="@var"
}

fn parse_false_attr(input: &str) -> VResult<'_, (bool, Vec<String>)> {
    let (input, _) = context("false attribute", tag("false")).parse(input)?;
    let (input, _) = cut(tag("=")).parse(input)?;
    let (input, (_, value)) = cut(context("false value", parse_quoted_value)).parse(input)?;

    let path = parse_condition_path(value).map_err(|e| make_error(input, e))?;

    Ok((input, (true, path))) // negate=true for false="@var"
}

// variable key inside attr quotes, ex: true="@foo.bar" -> ["foo", "bar"]
fn parse_condition_path(value: &str) -> Result<Vec<String>, String> {
    let path = value
        .strip_prefix('@')
        .ok_or_else(|| format!("If condition must start with @, got: \"{}\"", value))?;

    if path.is_empty() {
        return Err("If condition path cannot be empty after @".to_string());
    }

    Ok(path.split('.').map(String::from).collect())
}

// parse content until matching </If>, handling nested <If> tags
fn parse_until_closing_if(input: &str) -> Result<(&str, &str), String> {
    let mut depth = 1;
    let mut pos = 0;

    while pos < input.len() && depth > 0 {
        if input[pos..].starts_with("<If") {
            // Check if this is an opening tag (has whitespace after)
            let after_tag = &input[pos + 3..];
            if after_tag.starts_with(char::is_whitespace) {
                depth += 1;
            }
            pos += 3;
        } else if input[pos..].starts_with("</If>") {
            depth -= 1;
            if depth == 0 {
                let inner = &input[..pos];
                let remaining = &input[pos + 5..]; // Skip "</If>"
                return Ok((remaining, inner));
            }
            pos += 5;
        } else {
            pos += 1;
        }
    }
    Err("Unclosed <If> tag - missing </If>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_condition_path_tests {
        use super::*;

        #[test]
        fn simple_path() {
            let result = parse_condition_path("@active").unwrap();
            assert_eq!(result, vec!["active"]);
        }

        #[test]
        fn nested_path() {
            let result = parse_condition_path("@user.is_admin").unwrap();
            assert_eq!(result, vec!["user", "is_admin"]);
        }

        #[test]
        fn deeply_nested() {
            let result = parse_condition_path("@a.b.c.d").unwrap();
            assert_eq!(result, vec!["a", "b", "c", "d"]);
        }

        #[test]
        fn error_missing_at() {
            let result = parse_condition_path("active");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("must start with @"));
        }

        #[test]
        fn error_empty_after_at() {
            let result = parse_condition_path("@");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("cannot be empty"));
        }
    }

    mod parse_until_closing_tests {
        use super::*;

        #[test]
        fn simple_content() {
            let input = "some content</If>remaining";
            let (remaining, inner) = parse_until_closing_if(input).unwrap();
            assert_eq!(inner, "some content");
            assert_eq!(remaining, "remaining");
        }

        #[test]
        fn nested_if() {
            let input = "outer <If true=\"@x\">inner</If> more</If>after";
            let (remaining, inner) = parse_until_closing_if(input).unwrap();
            assert_eq!(inner, "outer <If true=\"@x\">inner</If> more");
            assert_eq!(remaining, "after");
        }

        #[test]
        fn deeply_nested() {
            let input = "<If true=\"@a\"><If false=\"@b\">deep</If></If></If>end";
            let (remaining, inner) = parse_until_closing_if(input).unwrap();
            assert_eq!(inner, "<If true=\"@a\"><If false=\"@b\">deep</If></If>");
            assert_eq!(remaining, "end");
        }

        #[test]
        fn error_unclosed() {
            let input = "content without closing tag";
            let result = parse_until_closing_if(input);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unclosed"));
        }
    }

    mod parse_if_tests {
        use super::*;

        #[test]
        fn simple_true_condition() {
            let input = r#"<If true="@active">content</If>after"#;
            let (remaining, node) = parse_if(input).unwrap();

            assert_eq!(remaining, "after");
            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.condition_path, vec!["active"]);
                    assert!(!if_node.negate);
                    assert_eq!(if_node.children.len(), 1);
                    assert!(matches!(&if_node.children[0], TemplateNode::Text(_)));
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn simple_false_condition() {
            let input = r#"<If false="@active">content</If>after"#;
            let (remaining, node) = parse_if(input).unwrap();

            assert_eq!(remaining, "after");
            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.condition_path, vec!["active"]);
                    assert!(if_node.negate);
                    assert_eq!(if_node.children.len(), 1);
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn nested_path() {
            let input = r#"<If true="@user.is_admin">admin</If>"#;
            let (_, node) = parse_if(input).unwrap();

            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.condition_path, vec!["user", "is_admin"]);
                    assert!(!if_node.negate);
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn with_variable_content() {
            let input = r#"<If true="@show">Hello @name</If>"#;
            let (_, node) = parse_if(input).unwrap();

            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.children.len(), 2);
                    assert!(matches!(&if_node.children[0], TemplateNode::Text(_)));
                    assert!(matches!(&if_node.children[1], TemplateNode::Variable(_)));
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn single_quotes() {
            let input = r#"<If true='@active'>content</If>"#;
            let (_, node) = parse_if(input).unwrap();

            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.condition_path, vec!["active"]);
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn nested_if_tags() {
            let input = r#"<If true="@outer"><If true="@inner">nested</If></If>after"#;
            let (remaining, node) = parse_if(input).unwrap();

            assert_eq!(remaining, "after");
            match node {
                TemplateNode::If(outer) => {
                    assert_eq!(outer.condition_path, vec!["outer"]);
                    assert_eq!(outer.children.len(), 1);
                    match &outer.children[0] {
                        TemplateNode::If(inner) => {
                            assert_eq!(inner.condition_path, vec!["inner"]);
                        }
                        _ => panic!("Expected nested If node"),
                    }
                }
                _ => panic!("Expected If node"),
            }
        }

        #[test]
        fn error_missing_at_prefix() {
            let input = r#"<If true="active">content</If>"#;
            let result = parse_if(input);
            assert!(result.is_err());
        }

        #[test]
        fn error_unclosed() {
            let input = r#"<If true="@active">content"#;
            let result = parse_if(input);
            assert!(result.is_err());
        }

        #[test]
        fn multiline_content() {
            let input = r#"<If true="@show">
  <div>Hello</div>
</If>"#;
            let (_, node) = parse_if(input).unwrap();

            match node {
                TemplateNode::If(if_node) => {
                    assert_eq!(if_node.condition_path, vec!["show"]);
                    assert!(if_node.children.len() >= 1);
                }
                _ => panic!("Expected If node"),
            }
        }
    }
}
