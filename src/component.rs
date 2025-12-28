#![allow(unused)]
use nom::bytes::complete::take_while;
use nom::character::complete::{anychar, char, satisfy, space0};
use nom::combinator::{cut, peek, recognize, rest, value};
use nom::multi::many_till;
use nom::sequence::delimited;
use nom::IResult;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, sequence::preceded};
use serde_json::Value;

use crate::BlazeTemplate;

// AST node for recursive descent parsing
#[derive(Debug)]
enum Node<'a> {
    Text(&'a str),
    Component {
        tag_name: &'a str,
        children: Vec<Node<'a>>,
    },
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

// Parse text until we hit a component start or closing tag
fn parse_text_until<'a>(input: &'a str, stop_tag: Option<&str>) -> IResult<&'a str, &'a str> {
    let mut end = 0;
    let bytes = input.as_bytes();

    while end < bytes.len() {
        if bytes[end] == b'<' {
            // Check for closing tag we're looking for
            if let Some(tag_name) = stop_tag {
                let close = format!("</{}>", tag_name);
                if input[end..].starts_with(&close) {
                    break;
                }
            }
            // Check for component start (uppercase)
            if bytes.get(end + 1).is_some_and(|b| b.is_ascii_uppercase()) {
                break;
            }
        }
        end += 1;
    }

    if end == 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TakeWhile1,
        )));
    }
    Ok((&input[end..], &input[..end]))
}

// Recursive component parser
fn parse_component(input: &str) -> IResult<&str, Node> {
    let (remaining, (tag_name, is_self_closing)) = parse_component_opening_tag(input)?;

    if is_self_closing {
        return Ok((
            remaining,
            Node::Component {
                tag_name,
                children: vec![],
            },
        ));
    }

    // RECURSIVE: parse inner content as nodes, stopping at our closing tag
    let (remaining, children) = parse_nodes(remaining, Some(tag_name))?;

    // Consume the closing tag - use cut to make missing tag a non-recoverable error
    let closing = format!("</{}>", tag_name);
    let (remaining, _) = cut(tag(closing.as_str())).parse(remaining)?;

    Ok((remaining, Node::Component { tag_name, children }))
}

// Recursive document parser - parses nodes until stop_tag or end of input
fn parse_nodes<'a>(input: &'a str, stop_at: Option<&str>) -> IResult<&'a str, Vec<Node<'a>>> {
    let mut nodes = Vec::new();
    let mut remaining = input;

    loop {
        // Check if we've hit the closing tag we're looking for
        if let Some(tag_name) = stop_at {
            let close = format!("</{}>", tag_name);
            if remaining.starts_with(&close) {
                break;
            }
        }

        // Check if input exhausted
        if remaining.is_empty() {
            break;
        }

        // Try to parse a component first
        match parse_component(remaining) {
            Ok((rest, node)) => {
                nodes.push(node);
                remaining = rest;
                continue;
            }
            Err(nom::Err::Failure(e)) => return Err(nom::Err::Failure(e)),
            Err(_) => {} // recoverable error, try text
        }

        // Otherwise parse text
        if let Ok((rest, text)) = parse_text_until(remaining, stop_at) {
            nodes.push(Node::Text(text));
            remaining = rest;
            continue;
        }

        // Handle case where we're at <Uppercase but it's not a valid component (e.g., <Layout with attrs>)
        // Consume one character as text and continue
        if !remaining.is_empty() {
            let mut char_len = 1;
            // Get the byte length of the first character
            if let Some(c) = remaining.chars().next() {
                char_len = c.len_utf8();
            }
            nodes.push(Node::Text(&remaining[..char_len]));
            remaining = &remaining[char_len..];
            continue;
        }

        break;
    }

    Ok((remaining, nodes))
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
    } else {
        Ok(template.to_string())
    }
}

// Render AST nodes to string, processing component templates
fn render_nodes(ctx: &BlazeTemplate, nodes: &[Node], data: &Value) -> Result<String, String> {
    let mut result = String::new();

    for node in nodes {
        match node {
            Node::Text(t) => result.push_str(t),
            Node::Component { tag_name, children } => {
                let path = ctx
                    .get_component_path(tag_name)
                    .ok_or_else(|| format!("Component '{}' not registered", tag_name))?;

                let template = ctx.read_template(path)?;

                // Render children to string for slot insertion
                let children_html = render_nodes(ctx, children, data)?;
                let with_slot = insert_slot_content(&template, &children_html)?;

                // Parse and render the component template (may contain more components)
                let (_, template_nodes) =
                    parse_nodes(&with_slot, None).map_err(|e| format!("Parse error: {}", e))?;
                let rendered = render_nodes(ctx, &template_nodes, data)?;

                result.push_str(&rendered);
            }
        }
    }

    Ok(result)
}

pub fn process_components(
    ctx: &BlazeTemplate,
    input: &str,
    data: &Value,
) -> Result<String, String> {
    // skip work if no Component exists
    if !might_contain_components(input) {
        return Ok(input.to_string());
    }

    let (_, nodes) = parse_nodes(input, None).map_err(|e| format!("Parse error: {}", e))?;

    render_nodes(ctx, &nodes, data)
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
    fn test_parse_nodes_self_closing() {
        let (remaining, nodes) = parse_nodes("before<Card />after", None).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], Node::Text("before")));
        assert!(matches!(
            &nodes[1],
            Node::Component {
                tag_name: "Card",
                children
            } if children.is_empty()
        ));
        assert!(matches!(nodes[2], Node::Text("after")));
    }

    #[test]
    fn test_parse_nodes_with_content() {
        let (remaining, nodes) = parse_nodes("before<Card>inner</Card>after", None).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], Node::Text("before")));
        if let Node::Component { tag_name, children } = &nodes[1] {
            assert_eq!(*tag_name, "Card");
            assert_eq!(children.len(), 1);
            assert!(matches!(children[0], Node::Text("inner")));
        } else {
            panic!("Expected Component node");
        }
        assert!(matches!(nodes[2], Node::Text("after")));
    }

    #[test]
    fn test_parse_nodes_no_match() {
        let (remaining, nodes) = parse_nodes("<div>content</div>", None).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0], Node::Text("<div>content</div>")));
    }

    #[test]
    fn test_parse_nodes_missing_closing_tag() {
        let result = parse_nodes("<Card>content", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_same_name_components() {
        let (_, nodes) = parse_nodes("<Card><Card>inner</Card>outer</Card>", None).unwrap();
        assert_eq!(nodes.len(), 1);
        if let Node::Component { tag_name, children } = &nodes[0] {
            assert_eq!(*tag_name, "Card");
            assert_eq!(children.len(), 2); // nested Card + "outer" text
            assert!(matches!(
                &children[0],
                Node::Component {
                    tag_name: "Card",
                    ..
                }
            ));
            assert!(matches!(children[1], Node::Text("outer")));
        } else {
            panic!("Expected Component node");
        }
    }

    #[test]
    fn test_deeply_nested_components() {
        let (_, nodes) = parse_nodes("<A><B><C>deep</C></B></A>", None).unwrap();
        assert_eq!(nodes.len(), 1);
        // Verify 3 levels of nesting
        if let Node::Component {
            tag_name: a,
            children: b_nodes,
        } = &nodes[0]
        {
            assert_eq!(*a, "A");
            if let Node::Component {
                tag_name: b,
                children: c_nodes,
            } = &b_nodes[0]
            {
                assert_eq!(*b, "B");
                if let Node::Component {
                    tag_name: c,
                    children: deep_nodes,
                } = &c_nodes[0]
                {
                    assert_eq!(*c, "C");
                    assert!(matches!(deep_nodes[0], Node::Text("deep")));
                } else {
                    panic!("Expected C component");
                }
            } else {
                panic!("Expected B component");
            }
        } else {
            panic!("Expected A component");
        }
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
