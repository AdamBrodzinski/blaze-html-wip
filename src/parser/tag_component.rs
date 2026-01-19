use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while_m_n};
use nom::character::complete::{multispace0, multispace1};
use nom::combinator::recognize;
use nom::error::{context, ErrorKind, ParseError};
use nom::multi::many0;
use nom::sequence::preceded;

use crate::ast::{ComponentNode, ComponentProp, PropValue, TemplateNode};

use super::error::{make_error, BlazeParseError, VResult};
use super::shared::attrs::parse_attr;

const RESERVED_COMPONENTS: [&str; 5] = ["Each", "If", "Script", "Style", "Slot"];

pub fn parse_component(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<").parse(input)?;
    let (input, name) = parse_component_name(input)?;

    if RESERVED_COMPONENTS.contains(&name) {
        return Err(nom::Err::Error(BlazeParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )));
    }

    let (input, attrs) = many0(preceded(multispace1, parse_attr)).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, is_self_closing) = parse_component_tag_end(input)?;

    let props = parse_props(attrs).map_err(|e| make_error(input, e))?;

    if is_self_closing {
        return Ok((
            input,
            TemplateNode::Component(ComponentNode {
                name: name.to_string(),
                props,
                children: Vec::new(),
            }),
        ));
    }

    let (remaining, inner_content) =
        parse_until_closing_component(input, name).map_err(|e| make_error(input, e))?;

    let children = super::parse_template_to_ast(inner_content)
        .map_err(|e| make_error(inner_content, format!("Error parsing component body: {}", e)))?;

    Ok((
        remaining,
        TemplateNode::Component(ComponentNode {
            name: name.to_string(),
            props,
            children,
        }),
    ))
}

pub fn parse_slot(input: &str) -> VResult<'_, TemplateNode> {
    let (input, _) = tag("<slot").parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = context("slot closing tag", tag("/>")).parse(input)?;
    Ok((input, TemplateNode::Slot))
}

fn parse_component_name(input: &str) -> VResult<'_, &str> {
    context(
        "component tag name",
        recognize((
            take_while_m_n(1, 1, |c: char| c.is_ascii_uppercase()),
            take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
        )),
    )
    .parse(input)
}

fn parse_component_tag_end(input: &str) -> VResult<'_, bool> {
    context(
        "component tag end",
        alt((tag("/>").map(|_| true), tag(">").map(|_| false))),
    )
    .parse(input)
}

fn parse_props(attrs: Vec<(&str, &str, char)>) -> Result<Vec<ComponentProp>, String> {
    let mut props = Vec::with_capacity(attrs.len());
    for (name, value, _) in attrs {
        let prop_value = if let Some(stripped) = value.strip_prefix('@') {
            if stripped.is_empty() {
                return Err(format!("component prop '{name}' cannot be empty after @"));
            }
            PropValue::VarPath(stripped.split('.').map(String::from).collect())
        } else {
            PropValue::Static(value.to_string())
        };

        props.push(ComponentProp {
            name: name.to_string(),
            value: prop_value,
        });
    }
    Ok(props)
}

fn parse_until_closing_component<'a>(input: &'a str, name: &str) -> Result<(&'a str, &'a str), String> {
    let mut depth = 1;
    let mut pos = 0;

    while pos < input.len() && depth > 0 {
        if input[pos..].starts_with("</") {
            if is_tag_at(&input[pos + 2..], name) {
                let close_pos = pos + 2 + name.len();
                if input[close_pos..].starts_with('>') {
                    depth -= 1;
                    if depth == 0 {
                        let inner = &input[..pos];
                        let remaining = &input[close_pos + 1..];
                        return Ok((remaining, inner));
                    }
                }
            }
        } else if input[pos..].starts_with('<') {
            if is_tag_at(&input[pos + 1..], name) {
                if let Some((is_self_closing, end_pos)) =
                    parse_tag_end_position(&input[pos + 1..], name)
                {
                    if !is_self_closing {
                        depth += 1;
                    }
                    pos += 1 + end_pos;
                    continue;
                }
            }
        }
        pos += 1;
    }

    Err(format!("Unclosed <{name}> tag - missing </{name}>"))
}

fn is_tag_at(input: &str, name: &str) -> bool {
    if !input.starts_with(name) {
        return false;
    }
    match input[name.len()..].chars().next() {
        Some(c) => c.is_whitespace() || c == '/' || c == '>',
        None => true,
    }
}

fn parse_tag_end_position(input: &str, name: &str) -> Option<(bool, usize)> {
    if !input.starts_with(name) {
        return None;
    }
    let mut pos = name.len();
    let mut in_single = false;
    let mut in_double = false;

    while pos < input.len() {
        let ch = input[pos..].chars().next()?;
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '>' if !in_single && !in_double => {
                let tag_body = &input[..pos];
                let is_self_closing = tag_body.trim_end().ends_with('/');
                return Some((is_self_closing, pos + 1));
            }
            _ => {}
        }
        pos += ch.len_utf8();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_self_closing_component() {
        let input = r#"<MyComponent foo="bar" />rest"#;
        let (remaining, node) = parse_component(input).unwrap();
        assert_eq!(remaining, "rest");
        match node {
            TemplateNode::Component(component) => {
                assert_eq!(component.name, "MyComponent");
                assert_eq!(component.props.len(), 1);
                assert!(component.children.is_empty());
            }
            _ => panic!("Expected Component node"),
        }
    }

    #[test]
    fn parses_component_with_children() {
        let input = r#"<Layout>Inner @name</Layout>after"#;
        let (remaining, node) = parse_component(input).unwrap();
        assert_eq!(remaining, "after");
        match node {
            TemplateNode::Component(component) => {
                assert_eq!(component.name, "Layout");
                assert_eq!(component.children.len(), 2);
            }
            _ => panic!("Expected Component node"),
        }
    }

    #[test]
    fn parses_prop_variable_value() {
        let input = r#"<Card title="@person.name" />"#;
        let (_, node) = parse_component(input).unwrap();
        match node {
            TemplateNode::Component(component) => {
                assert!(matches!(
                    component.props[0].value,
                    PropValue::VarPath(_)
                ));
            }
            _ => panic!("Expected Component node"),
        }
    }

    #[test]
    fn parse_slot_tag() {
        let input = "<slot/>rest";
        let (remaining, node) = parse_slot(input).unwrap();
        assert_eq!(remaining, "rest");
        assert!(matches!(node, TemplateNode::Slot));
    }
}
