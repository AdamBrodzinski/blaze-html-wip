//! Each tag parser for iterating over JSON arrays
//!
//! Parses `<Each items="@collection" as="item">...</Each>` tags and creates
//! an EachNode containing the parsed children. JSON data is scoped per each block

use nom::Parser;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::cut;
use nom::error::context;

use crate::ast::{EachNode, TemplateNode};
use crate::parser_error::{VResult, make_error};
use crate::shared_parsers::attrs::parse_quoted_value;

pub fn parse_each(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<Each").parse(input)?;
    let (input, _) = multispace1.parse(input)?;

    let (input, _) = context("items attribute", tag("items")).parse(input)?;
    let (input, _) = cut(tag("=")).parse(input)?;
    let (input, (_, items_value)) = cut(context("items value", parse_quoted_value)).parse(input)?;

    let items_path = parse_items_path(items_value).map_err(|e| make_error(input, e))?;

    let (input, _) = multispace1.parse(input)?;

    let (input, _) = context("as attribute", tag("as")).parse(input)?;
    let (input, _) = cut(tag("=")).parse(input)?;
    let (input, (_, as_value)) = cut(context("as value", parse_quoted_value)).parse(input)?;

    // as attribute must be simple identifier, no dots
    let item_binding = validate_binding(as_value).map_err(|e| make_error(input, e))?;

    let (input, _) = multispace0.parse(input)?;

    let (input, _) = context("each opening tag >", cut(tag(">"))).parse(input)?;

    // walk forward to the matching </Each>, accounting for nested Each tags
    let (remaining, inner_content) =
        parse_until_closing_each(input).map_err(|e| make_error(input, e))?;

    // parse inner content recursively
    let children = crate::parse::parse_template_to_ast(inner_content)
        .map_err(|e| make_error(inner_content, format!("Error parsing Each body: {}", e)))?;

    Ok((
        remaining,
        TemplateNode::Each(EachNode {
            items_path,
            item_binding,
            children,
        }),
    ))
}

fn parse_items_path(value: &str) -> Result<Vec<String>, String> {
    let path = value
        .strip_prefix('@')
        .ok_or_else(|| format!("items value must start with @, got: \"{}\"", value))?;

    if path.is_empty() {
        return Err("items path cannot be empty after @".to_string());
    }

    Ok(path.split('.').map(String::from).collect())
}

fn validate_binding(value: &str) -> Result<String, String> {
    let first_char = value
        .chars()
        .next()
        .ok_or_else(|| "as binding cannot be empty".to_string())?;

    if !first_char.is_alphabetic() && first_char != '_' {
        return Err(format!(
            "'as' binding must start with letter or underscore, got: \"{}\"",
            value
        ));
    }

    for c in value.chars() {
        if c == '.' {
            return Err(format!(
                "'as' binding must be a simple identifier without dots, got: \"{}\"",
                value
            ));
        }
        if !c.is_alphanumeric() && c != '_' {
            return Err(format!(
                "'as' binding contains invalid character '{}' in: \"{}\"",
                c, value
            ));
        }
    }

    Ok(value.to_string())
}

// TODO: simplify search
// parse content until matching </Each>, handling nested <Each> tags
fn parse_until_closing_each(input: &str) -> Result<(&str, &str), String> {
    let mut depth = 1;
    let mut pos = 0;

    while pos < input.len() && depth > 0 {
        if input[pos..].starts_with("<Each") {
            // Check if this is an opening tag (has whitespace and attributes after)
            let after_tag = &input[pos + 5..];
            if after_tag.starts_with(char::is_whitespace) {
                depth += 1;
            }
            pos += 5;
        } else if input[pos..].starts_with("</Each>") {
            depth -= 1;
            if depth == 0 {
                let inner = &input[..pos];
                let remaining = &input[pos + 7..]; // Skip "</Each>"
                return Ok((remaining, inner));
            }
            pos += 7;
        } else {
            pos += 1;
        }
    }
    Err("Unclosed <Each> tag - missing </Each>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_items_path_tests {
        use super::*;

        #[test]
        fn simple_path() {
            let result = parse_items_path("@people").unwrap();
            assert_eq!(result, vec!["people"]);
        }

        #[test]
        fn nested_path() {
            let result = parse_items_path("@person.skills").unwrap();
            assert_eq!(result, vec!["person", "skills"]);
        }

        #[test]
        fn deeply_nested() {
            let result = parse_items_path("@a.b.c.d").unwrap();
            assert_eq!(result, vec!["a", "b", "c", "d"]);
        }

        #[test]
        fn error_missing_at() {
            let result = parse_items_path("people");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("must start with @"));
        }

        #[test]
        fn error_empty_after_at() {
            let result = parse_items_path("@");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("cannot be empty"));
        }
    }

    mod validate_binding_tests {
        use super::*;

        #[test]
        fn simple_binding() {
            let result = validate_binding("person").unwrap();
            assert_eq!(result, "person");
        }

        #[test]
        fn underscore_binding() {
            let result = validate_binding("_item").unwrap();
            assert_eq!(result, "_item");
        }

        #[test]
        fn alphanumeric_binding() {
            let result = validate_binding("item2").unwrap();
            assert_eq!(result, "item2");
        }

        #[test]
        fn error_empty() {
            let result = validate_binding("");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("cannot be empty"));
        }

        #[test]
        fn error_with_dots() {
            let result = validate_binding("person.name");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("without dots"));
        }

        #[test]
        fn error_starts_with_number() {
            let result = validate_binding("2item");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("must start with letter"));
        }
    }

    mod parse_until_closing_tests {
        use super::*;

        #[test]
        fn simple_content() {
            let input = "some content</Each>remaining";
            let (remaining, inner) = parse_until_closing_each(input).unwrap();
            assert_eq!(inner, "some content");
            assert_eq!(remaining, "remaining");
        }

        #[test]
        fn nested_each() {
            let input = "outer <Each items=\"@x\" as=\"y\">inner</Each> more</Each>after";
            let (remaining, inner) = parse_until_closing_each(input).unwrap();
            assert_eq!(inner, "outer <Each items=\"@x\" as=\"y\">inner</Each> more");
            assert_eq!(remaining, "after");
        }

        #[test]
        fn deeply_nested() {
            let input = "<Each items=\"@a\" as=\"b\"><Each items=\"@c\" as=\"d\">deep</Each></Each></Each>end";
            let (remaining, inner) = parse_until_closing_each(input).unwrap();
            assert_eq!(
                inner,
                "<Each items=\"@a\" as=\"b\"><Each items=\"@c\" as=\"d\">deep</Each></Each>"
            );
            assert_eq!(remaining, "end");
        }

        #[test]
        fn error_unclosed() {
            let input = "content without closing tag";
            let result = parse_until_closing_each(input);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unclosed"));
        }
    }

    mod parse_each_tests {
        use super::*;

        #[test]
        fn simple_each() {
            let input = r#"<Each items="@people" as="person">@person.name</Each>after"#;
            let (remaining, node) = parse_each(input).unwrap();

            assert_eq!(remaining, "after");
            match node {
                TemplateNode::Each(each) => {
                    assert_eq!(each.items_path, vec!["people"]);
                    assert_eq!(each.item_binding, "person");
                    assert_eq!(each.children.len(), 1);
                    assert!(matches!(&each.children[0], TemplateNode::Variable(_)));
                }
                _ => panic!("Expected Each node"),
            }
        }

        #[test]
        fn nested_path() {
            let input = r#"<Each items="@user.skills" as="skill">@skill</Each>"#;
            let (_, node) = parse_each(input).unwrap();

            match node {
                TemplateNode::Each(each) => {
                    assert_eq!(each.items_path, vec!["user", "skills"]);
                    assert_eq!(each.item_binding, "skill");
                }
                _ => panic!("Expected Each node"),
            }
        }

        #[test]
        fn with_text_content() {
            let input = r#"<Each items="@items" as="item">Name: @item.name</Each>"#;
            let (_, node) = parse_each(input).unwrap();

            match node {
                TemplateNode::Each(each) => {
                    assert_eq!(each.children.len(), 2);
                    assert!(matches!(&each.children[0], TemplateNode::Text(_)));
                    assert!(matches!(&each.children[1], TemplateNode::Variable(_)));
                }
                _ => panic!("Expected Each node"),
            }
        }

        #[test]
        fn multiline() {
            let input = r#"<Each items="@people" as="p">
  <div>@p.name</div>
</Each>"#;
            let (_, node) = parse_each(input).unwrap();

            match node {
                TemplateNode::Each(each) => {
                    assert_eq!(each.items_path, vec!["people"]);
                    assert_eq!(each.item_binding, "p");
                    // Should have text nodes and variable
                    assert!(each.children.len() >= 1);
                }
                _ => panic!("Expected Each node"),
            }
        }

        #[test]
        fn single_quotes() {
            let input = r#"<Each items='@items' as='item'>content</Each>"#;
            let (_, node) = parse_each(input).unwrap();

            match node {
                TemplateNode::Each(each) => {
                    assert_eq!(each.items_path, vec!["items"]);
                    assert_eq!(each.item_binding, "item");
                }
                _ => panic!("Expected Each node"),
            }
        }

        #[test]
        fn error_missing_at_prefix() {
            let input = r#"<Each items="people" as="person">content</Each>"#;
            let result = parse_each(input);
            assert!(result.is_err());
        }

        #[test]
        fn error_wrong_attribute_order() {
            let input = r#"<Each as="person" items="@people">content</Each>"#;
            let result = parse_each(input);
            assert!(result.is_err());
        }

        #[test]
        fn error_binding_with_dots() {
            let input = r#"<Each items="@people" as="person.name">content</Each>"#;
            let result = parse_each(input);
            assert!(result.is_err());
        }

        #[test]
        fn error_unclosed() {
            let input = r#"<Each items="@people" as="person">content"#;
            let result = parse_each(input);
            assert!(result.is_err());
        }
    }
}
