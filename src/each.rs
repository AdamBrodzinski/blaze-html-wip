#![allow(unused)]

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until, take_while1};
use nom::character::complete::{anychar, char, space0, space1};
use nom::combinator::{not, opt, peek, recognize, rest};
use nom::multi::{many0, many1};
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;
use serde_json::Value;

use crate::data::get_json_value;
use crate::parsers::parse_quoted_value;
use crate::variables::process_variables;

// ---------------------- AST Types ----------------------

#[derive(Debug, PartialEq)]
enum Node<'a> {
    Text(&'a str),
    Each {
        tag: EachTag<'a>,
        children: Vec<Node<'a>>,
    },
}

#[derive(Debug, PartialEq)]
struct EachTag<'a> {
    items_path: &'a str,
    as_var: &'a str,
    idx_var: Option<&'a str>,
}

// ---------------------- Text Parser ----------------------

// Check if we're at the start of an Each tag
fn at_each_tag(input: &str) -> bool {
    input.starts_with("<Each") || input.starts_with("</Each>")
}

// Parse a single text char/segment
fn text_char(input: &str) -> IResult<&str, char> {
    // Match any char that isn't < at an Each tag boundary
    if input.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let c = input.chars().next().unwrap();

    // If it's not '<', consume it
    if c != '<' {
        return Ok((&input[c.len_utf8()..], c));
    }

    // It's '<' - check if it's an Each tag
    if at_each_tag(input) {
        // Don't consume - let the each parser handle it
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // It's '<' but not an Each tag - consume it
    Ok((&input[1..], '<'))
}

fn text(input: &str) -> IResult<&str, Node<'_>> {
    let (rest, txt) = take_till(|c| c == '<')(input)?;

    if txt.is_empty() {
        // We are at '<' — decide whether this is Each or literal '<'
        if at_each_tag(input) {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }

        // Consume the '<' as text
        let (rest, _) = char('<')(input)?;
        return Ok((rest, Node::Text("<")));
    }

    Ok((rest, Node::Text(txt)))
}

// ---------------------- Attribute Parsers ----------------------

fn parse_items_attribute(input: &str) -> IResult<&str, &str> {
    let (input, (_, _, _, _, value)) =
        (tag("items"), space0, char('='), space0, parse_quoted_value).parse(input)?;

    Ok((input, value.strip_prefix('@').unwrap_or(value)))
}

fn parse_as_attribute(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("as").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    parse_quoted_value(input)
}

fn parse_idx_attribute(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("idx").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    parse_quoted_value(input)
}

fn parse_each_opening_tag(input: &str) -> IResult<&str, EachTag<'_>> {
    let (input, _) = tag("<Each").parse(input)?;
    let (input, _) = space1(input)?;
    let (input, items_path) = parse_items_attribute(input)?;
    let (input, _) = space1(input)?;
    let (input, as_var) = parse_as_attribute(input)?;
    let (input, _) = space0(input)?;
    let (input, idx_var) = opt(parse_idx_attribute).parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('>').parse(input)?;
    Ok((
        input,
        EachTag {
            items_path,
            as_var,
            idx_var,
        },
    ))
}

// ---------------------- Recursive Descent Parsers ----------------------

fn each(input: &str) -> IResult<&str, Node<'_>> {
    let (input, tag_info) = parse_each_opening_tag(input)?;
    let (input, children) = many0(content).parse(input)?;
    let (input, _) = tag("</Each>").parse(input)?;

    Ok((
        input,
        Node::Each {
            tag: tag_info,
            children,
        },
    ))
}

fn content(input: &str) -> IResult<&str, Node<'_>> {
    alt((each, text)).parse(input)
}

fn parse_template(input: &str) -> IResult<&str, Vec<Node<'_>>> {
    many0(content).parse(input)
}

// ---------------------- AST Processing ----------------------

fn build_iteration_context(original: &Value, item: &Value, index: usize, tag: &EachTag) -> Value {
    let mut ctx = original.clone();
    if let Value::Object(ref mut map) = ctx {
        map.insert(tag.as_var.to_string(), item.clone());
        if let Some(idx) = tag.idx_var {
            map.insert(idx.to_string(), Value::Number(index.into()));
        }
    }
    ctx
}

fn process_nodes(nodes: &[Node], data: &Value) -> Result<String, String> {
    let mut output = String::new();
    for node in nodes {
        match node {
            Node::Text(txt) => {
                // Process variables in text with current data context
                let processed = process_variables(txt, data)?;
                output.push_str(&processed);
            }
            Node::Each { tag, children } => {
                let expanded = process_each_node(tag, children, data)?;
                output.push_str(&expanded);
            }
        }
    }
    Ok(output)
}

fn process_each_node(tag: &EachTag, children: &[Node], data: &Value) -> Result<String, String> {
    // Get array from data
    let array_value = get_json_value(data, tag.items_path)?;
    let array = match array_value {
        Value::Array(arr) => arr,
        _ => return Err(format!("'{}' is not an array", tag.items_path)),
    };

    // Iterate and build output
    let mut output = String::new();
    for (index, item) in array.iter().enumerate() {
        let ctx = build_iteration_context(data, item, index + 1, tag); // 1-based index
        let processed = process_nodes(children, &ctx)?;
        output.push_str(&processed);
    }
    Ok(output)
}

// ---------------------- Main Entry Point ----------------------

pub fn process_each(input: &str, data: &Value) -> Result<String, String> {
    // Quick check
    if !input.contains("<Each") {
        return Ok(input.to_string());
    }

    // Parse into AST
    let (_, nodes) = parse_template(input).map_err(|e| format!("Parse error: {e}"))?;

    // Process AST nodes
    process_nodes(&nodes, data)
}

// ---------------------- Tests ----------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn basic_array_of_objects() {
        let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
        let tmpl = r#"<Each items="@people" as="p">@p.name,</Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "Alice,Bob,");
    }

    #[test]
    fn with_index_1_based() {
        let data = json!({"items": [{"x": "A"}, {"x": "B"}]});
        let tmpl = r#"<Each items="@items" as="item" idx="i">@i:@item.x </Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "1:A 2:B ");
    }

    #[test]
    fn nested_loops() {
        let data = json!({
            "groups": [
                {"name": "G1", "items": [{"v": 1}, {"v": 2}]},
                {"name": "G2", "items": [{"v": 3}]}
            ]
        });
        let tmpl = r#"<Each items="@groups" as="g">[@g.name:<Each items="@g.items" as="i">@i.v</Each>]</Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "[G1:12][G2:3]");
    }

    #[test]
    fn empty_array_produces_nothing() {
        let data = json!({"items": []});
        let tmpl = r#"<Each items="@items" as="x">@x</Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "");
    }

    #[test]
    fn array_of_primitives() {
        let data = json!({"names": ["Alice", "Bob"]});
        let tmpl = r#"<Each items="@names" as="n">@n,</Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "Alice,Bob,");
    }

    #[test]
    fn missing_array_errors() {
        let data = json!({});
        let tmpl = r#"<Each items="@missing" as="x">@x</Each>"#;
        assert!(process_each(tmpl, &data).is_err());
    }

    #[test]
    fn non_array_errors() {
        let data = json!({"items": "not array"});
        let tmpl = r#"<Each items="@items" as="x">@x</Each>"#;
        assert!(process_each(tmpl, &data).is_err());
    }

    #[test]
    fn sibling_loops() {
        let data = json!({"a": [1, 2], "b": [3, 4]});
        let tmpl = r#"<Each items="@a" as="x">@x</Each>-<Each items="@b" as="y">@y</Each>"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "12-34");
    }

    #[test]
    fn no_each_tag_returns_input() {
        let data = json!({});
        let tmpl = "Hello World";
        assert_eq!(process_each(tmpl, &data).unwrap(), "Hello World");
    }

    #[test]
    fn text_before_and_after() {
        let data = json!({"items": [{"x": 1}]});
        let tmpl = r#"before<Each items="@items" as="i">@i.x</Each>after"#;
        assert_eq!(process_each(tmpl, &data).unwrap(), "before1after");
    }
}
