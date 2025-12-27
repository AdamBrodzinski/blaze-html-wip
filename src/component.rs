#![allow(unused)]
use nom::bytes::complete::take_while;
use nom::character::complete::{anychar, char, satisfy, space0};
use nom::combinator::{peek, recognize, rest, value};
use nom::multi::many_till;
use nom::sequence::delimited;
use nom::IResult;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, sequence::preceded};
use serde_json::Value;

use crate::BlazeTemplate;

struct ComponentInstance<'a> {
    before: &'a str,
    tag_name: &'a str,
    inner_content: &'a str,
    after: &'a str,
}

struct SlotParts<'a> {
    before_slot: &'a str,
    after_slot: &'a str,
}

// prevents parser from failing if it gets to the end and nothing was found
fn might_contain_components(input: &str) -> bool {
    let bytes = input.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'<' && bytes.get(i + 1).is_some_and(|b| b.is_ascii_uppercase()) {
            return true;
        }
    }
    false
}

// components use the syntax <Foo>, where any uppercase tag is assumed to be a component
fn uppercase_tag_name(input: &str) -> IResult<&str, &str> {
    recognize((
        satisfy(|c: char| c.is_ascii_uppercase()),
        take_while(|c: char| c.is_alphanumeric()),
    ))
    .parse(input)
}

// parse both <Card>inner</Card or self closing tag <Card />
fn parse_component_opening_tag(input: &str) -> IResult<&str, (&str, bool)> {
    let (input, _) = char('<').parse(input)?;
    let (input, tag_name) = uppercase_tag_name(input)?;
    let (input, _) = space0(input)?;
    let (input, is_self_closing) =
        alt((value(true, tag("/>")), value(false, char('>')))).parse(input)?;
    Ok((input, (tag_name, is_self_closing)))
}

fn extract_first_component(input: &str) -> Result<Option<ComponentInstance>, String> {
    let bytes = input.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        // Find next '<'
        let Some(rel_idx) = input[pos..].find('<') else {
            break;
        };
        let actual_idx = pos + rel_idx;
        let remaining = &input[actual_idx..];

        // Check if next char is uppercase
        if remaining.len() < 2 || !remaining.as_bytes()[1].is_ascii_uppercase() {
            pos = actual_idx + 1;
            continue;
        }

        // Try to parse as component opening tag
        if let Ok((after_open, (tag_name, is_self_closing))) =
            parse_component_opening_tag(remaining)
        {
            let before = &input[..actual_idx];

            if is_self_closing {
                return Ok(Some(ComponentInstance {
                    before,
                    tag_name,
                    inner_content: "",
                    after: after_open,
                }));
            } else {
                // Find closing tag </TagName>
                let closing_tag = format!("</{}>", tag_name);
                if let Some(close_idx) = after_open.find(&closing_tag) {
                    let inner = &after_open[..close_idx];
                    let after = &after_open[close_idx + closing_tag.len()..];
                    return Ok(Some(ComponentInstance {
                        before,
                        tag_name,
                        inner_content: inner,
                        after,
                    }));
                }
                return Err(format!("Missing closing tag </{}>", tag_name));
            }
        }
        pos = actual_idx + 1;
    }
    Ok(None)
}

fn slot_tag(input: &str) -> IResult<&str, ()> {
    value(
        (),
        (char('<'), space0, tag("slot"), space0, char('/'), char('>')),
    )
    .parse(input)
}

fn extract_slot_parts(input: &str) -> IResult<&str, SlotParts> {
    let (input, start) = recognize(many_till(anychar, peek(slot_tag))).parse(input)?;
    let (input, _) = slot_tag(input)?;
    let (input, end) = rest(input)?;
    Ok((
        input,
        SlotParts {
            before_slot: start,
            after_slot: end,
        },
    ))
}

fn insert_slot_content(template: &str, inner_content: &str) -> Result<String, String> {
    if let Ok((_, slot_parts)) = extract_slot_parts(template) {
        Ok(format!(
            "{}{}{}",
            slot_parts.before_slot, inner_content, slot_parts.after_slot
        ))
        // TODO possible edge case where it has a slot and fails?
    } else {
        Ok(template.to_string())
    }
}

pub fn process_components(
    ctx: &BlazeTemplate,
    input: &str,
    _data: &Value,
) -> Result<String, String> {
    // skip work if no Component exists, also **recursive base case**
    if !might_contain_components(input) {
        return Ok(input.to_string());
    }

    let Some(component) = extract_first_component(input)? else {
        return Ok(input.to_string());
    };

    let path = ctx
        .get_component_path(component.tag_name)
        .ok_or_else(|| format!("Component '{}' not registered", component.tag_name))?;

    let template = ctx.read_template(path)?;

    let replacement = insert_slot_content(&template, component.inner_content)?;

    let result = format!("{}{}{}", component.before, replacement, component.after);
    // recursively process all component tags
    process_components(ctx, &result, _data)
}

// note, see build_template.rs for feature tests
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_might_contain_components() {
        assert!(might_contain_components("<Card>"));
        assert!(might_contain_components("text<Button/>more"));
        assert!(!might_contain_components("<div>"));
        assert!(!might_contain_components("plain text"));
        assert!(!might_contain_components("<card>")); // lowercase
    }

    #[test]
    fn test_uppercase_tag_name() {
        assert_eq!(uppercase_tag_name("Card>"), Ok((">", "Card")));
        assert_eq!(uppercase_tag_name("Button123 />"), Ok((" />", "Button123")));
        assert!(uppercase_tag_name("card>").is_err()); // lowercase fails
    }

    #[test]
    fn test_parse_component_opening_tag() {
        assert_eq!(
            parse_component_opening_tag("<Card>"),
            Ok(("", ("Card", false)))
        );
        assert_eq!(
            parse_component_opening_tag("<Card />"),
            Ok(("", ("Card", true)))
        );
        assert_eq!(
            parse_component_opening_tag("<Card/>"),
            Ok(("", ("Card", true)))
        );
        assert_eq!(
            parse_component_opening_tag("<Button>rest"),
            Ok(("rest", ("Button", false)))
        );
    }

    #[test]
    fn test_extract_first_component_self_closing() {
        let result = extract_first_component("before<Card />after")
            .unwrap()
            .unwrap();
        assert_eq!(result.before, "before");
        assert_eq!(result.tag_name, "Card");
        assert_eq!(result.inner_content, "");
        assert_eq!(result.after, "after");
    }

    #[test]
    fn test_extract_first_component_with_content() {
        let result = extract_first_component("before<Card>inner</Card>after")
            .unwrap()
            .unwrap();
        assert_eq!(result.before, "before");
        assert_eq!(result.tag_name, "Card");
        assert_eq!(result.inner_content, "inner");
        assert_eq!(result.after, "after");
    }

    #[test]
    fn test_extract_first_component_no_match() {
        let result = extract_first_component("<div>content</div>").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_first_component_missing_closing_tag() {
        let result = extract_first_component("<Card>content");
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_slot_content() {
        let template = "Header <slot /> Footer";
        let result = insert_slot_content(template, "Content").unwrap();
        assert_eq!(result, "Header Content Footer");
    }

    #[test]
    fn test_insert_slot_content_no_slot() {
        let template = "Static content";
        let result = insert_slot_content(template, "Ignored").unwrap();
        assert_eq!(result, "Static content");
    }

    #[test]
    fn test_slot_tag_variations() {
        assert!(slot_tag("<slot/>").is_ok());
        assert!(slot_tag("< slot />").is_ok());
        assert!(slot_tag("<  slot   />").is_ok());
    }
}
