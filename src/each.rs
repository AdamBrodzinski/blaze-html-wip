#![allow(unused)]

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_until, take_while1};
use nom::character::complete::{char, space0, space1};
use nom::combinator::{rest, verify};
use nom::{AsBytes, IResult, Parser};
use serde_json::Value;

use crate::data::get_json_value;
use crate::variables::process_variables;

type Bytes = [u8];
type Document<'a> = Vec<Node<'a>>;

#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    Text(&'a Bytes),
    Each {
        items_path: &'a str,
        children: Vec<Node<'a>>,
    },
}

pub fn process_each(input: &str, data: &Value) -> Result<String, String> {
    let (_, nodes) = document(input.as_bytes()).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(input.len());
    render_nodes(&mut out, &nodes, data, None)?;
    String::from_utf8(out).map_err(|e| e.to_string())
}

fn build_iteration_context(original: &Value, item: &Value, index: usize) -> Value {
    let mut ctx = original.clone();
    if let Value::Object(ref mut map) = ctx {
        map.insert("item".to_string(), item.clone());
        map.insert("i".to_string(), Value::Number(index.into()));
    }
    ctx
}

fn render_nodes(
    out: &mut Vec<u8>,
    nodes: &[Node],
    data: &Value,
    current_item: Option<(&Value, usize)>, // (item, 1-based index)
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Text(txt) => {
                let text = std::str::from_utf8(txt).map_err(|e| e.to_string())?;
                let ctx = match current_item {
                    Some((item, idx)) => build_iteration_context(data, item, idx),
                    None => data.clone(),
                };
                let processed = process_variables(text, &ctx)?;
                out.extend_from_slice(processed.as_bytes());
            }
            Node::Each {
                items_path,
                children,
            } => {
                let array_value = get_json_value(data, items_path)?;
                let array = match array_value {
                    Value::Array(arr) => arr,
                    _ => return Err(format!("'{}' is not an array", items_path)),
                };

                for (idx, item) in array.iter().enumerate() {
                    render_nodes(out, children, data, Some((item, idx + 1)))?; // 1-based index
                }
            }
        }
    }
    Ok(())
}

pub fn document(input: &'_ Bytes) -> IResult<&'_ Bytes, Vec<Node<'_>>> {
    nom::multi::many0(node).parse(input)
}

fn node(input: &'_ Bytes) -> IResult<&'_ Bytes, Node<'_>> {
    alt((each_node, text_node)).parse(input)
}

fn each_node(input: &Bytes) -> IResult<&Bytes, Node<'_>> {
    let (input, items_path) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, _) = close_each(input)?;
    Ok((
        input,
        Node::Each {
            items_path,
            children,
        },
    ))
}

// optimize the text node case by manually parsing
fn text_node(input: &Bytes) -> IResult<&Bytes, Node<'_>> {
    use nom::error::{Error, ErrorKind};
    let len = input.len();
    let mut i = 0;
    while i < len {
        if input[i] == b'<' {
            let rest = &input[i..];
            // Check for <Each with space (for attributes) or </Each>
            if rest.starts_with(b"<Each ") || rest.starts_with(b"</Each>") {
                break;
            }
        }
        i += 1;
    }
    if i == 0 {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::TakeTill1)));
    }
    let (matched, rest) = input.split_at(i);
    Ok((rest, Node::Text(matched)))
}

/// Parses `<Each items="@path">` and returns the path (without @ prefix)
fn open_each(input: &Bytes) -> IResult<&Bytes, &str> {
    use nom::error::{Error, ErrorKind};

    // Parse <Each
    let (input, _) = tag(b"<Each " as &Bytes).parse(input)?;

    // Skip optional extra whitespace (already consumed one space in the tag)
    let input = skip_spaces(input);

    // Parse items=
    let (input, _) = tag(b"items=" as &Bytes).parse(input)?;

    // Parse quoted value (single or double quotes)
    let quote_char = input
        .first()
        .ok_or_else(|| nom::Err::Error(Error::new(input, ErrorKind::Char)))?;

    if *quote_char != b'"' && *quote_char != b'\'' {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::Char)));
    }

    let (input, _) = tag(&[*quote_char][..]).parse(input)?;

    // Find closing quote
    let close_pos = input
        .iter()
        .position(|&c| c == *quote_char)
        .ok_or_else(|| nom::Err::Error(Error::new(input, ErrorKind::TakeUntil)))?;

    let (value, rest) = input.split_at(close_pos);
    let (rest, _) = tag(&[*quote_char][..]).parse(rest)?;

    // Skip optional whitespace and closing >
    let rest = skip_spaces(rest);
    let (rest, _) = tag(b">" as &Bytes).parse(rest)?;

    // Convert value to str and strip @ prefix
    let value_str = std::str::from_utf8(value)
        .map_err(|_| nom::Err::Error(Error::new(input, ErrorKind::Char)))?;

    let path = value_str.strip_prefix('@').unwrap_or(value_str);

    Ok((rest, path))
}

fn skip_spaces(input: &Bytes) -> &Bytes {
    let mut i = 0;
    while i < input.len() && input[i] == b' ' {
        i += 1;
    }
    &input[i..]
}

fn close_each(input: &Bytes) -> IResult<&Bytes, &Bytes> {
    tag(b"</Each>" as &Bytes).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod process {
        use super::*;

        #[test]
        fn template_without_each() {
            let data = json!({"items": [1, 2, 3]});
            let result = process_each(r#"<div>Foo</div>"#, &data).unwrap();
            assert_eq!(result, "<div>Foo</div>");
        }

        #[test]
        fn basic_index_substitution() {
            let data = json!({"items": [1, 2, 3]});
            let result = process_each(r#"<Each items="@items">@i </Each>"#, &data).unwrap();
            assert_eq!(result, "1 2 3 ");
        }

        #[test]
        fn index_with_text() {
            let data = json!({"items": ["a", "b"]});
            let result = process_each(r#"<Each items="@items">@i Foo </Each>"#, &data).unwrap();
            assert_eq!(result, "1 Foo 2 Foo ");
        }

        #[test]
        fn text_before_and_after() {
            let data = json!({"items": [1, 2]});
            let result =
                process_each(r#"Before <Each items="@items">@i</Each> After"#, &data).unwrap();
            assert_eq!(result, "Before 12 After");
        }

        #[test]
        fn empty_array_produces_nothing() {
            let data = json!({"items": []});
            let result = process_each(r#"<Each items="@items">@i</Each>"#, &data).unwrap();
            assert_eq!(result, "");
        }

        #[test]
        fn no_each_tag_returns_input() {
            let data = json!({});
            let result = process_each("Hello World", &data).unwrap();
            assert_eq!(result, "Hello World");
        }

        #[test]
        fn nested_path() {
            let data = json!({"data": {"items": [1, 2]}});
            let result = process_each(r#"<Each items="@data.items">@i</Each>"#, &data).unwrap();
            assert_eq!(result, "12");
        }

        #[test]
        fn single_quotes() {
            let data = json!({"items": [1, 2]});
            let result = process_each(r#"<Each items='@items'>@i</Each>"#, &data).unwrap();
            assert_eq!(result, "12");
        }

        #[test]
        fn missing_array_errors() {
            let data = json!({});
            let result = process_each(r#"<Each items="@missing">@i</Each>"#, &data);
            assert!(result.is_err());
        }

        #[test]
        fn non_array_errors() {
            let data = json!({"items": "not an array"});
            let result = process_each(r#"<Each items="@items">@i</Each>"#, &data);
            assert!(result.is_err());
        }

        #[test]
        fn item_access_primitives() {
            let data = json!({"items": ["Alice", "Bob"]});
            let result = process_each(r#"<Each items="@items">@item,</Each>"#, &data).unwrap();
            assert_eq!(result, "Alice,Bob,");
        }

        #[test]
        fn item_access_objects() {
            let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
            let result =
                process_each(r#"<Each items="@people">@item.name,</Each>"#, &data).unwrap();
            assert_eq!(result, "Alice,Bob,");
        }

        #[test]
        fn item_and_index_together() {
            let data = json!({"items": ["a", "b", "c"]});
            let result = process_each(r#"<Each items="@items">@i:@item </Each>"#, &data).unwrap();
            assert_eq!(result, "1:a 2:b 3:c ");
        }
    }

    mod text {
        use super::*;

        #[test]
        fn text_and_each_tag() {
            let (remaining, text) = text_node(b"foo <Each items=\"@x\">").unwrap();
            assert_eq!(text, Node::Text(b"foo "));
            assert_eq!(remaining, b"<Each items=\"@x\">");
        }

        #[test]
        fn text_only() {
            let (remaining, text) = text_node(b"foo bar").unwrap();
            assert_eq!(text, Node::Text(b"foo bar"));
            assert_eq!(remaining, b"");
        }
    }

    mod open_each_tag {
        use super::*;

        #[test]
        fn parses_items_double_quotes() {
            let (remaining, path) = open_each(b"<Each items=\"@people\">rest").unwrap();
            assert_eq!(path, "people");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_items_single_quotes() {
            let (remaining, path) = open_each(b"<Each items='@users'>rest").unwrap();
            assert_eq!(path, "users");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_nested_path() {
            let (remaining, path) = open_each(b"<Each items=\"@data.items\">rest").unwrap();
            assert_eq!(path, "data.items");
            assert_eq!(remaining, b"rest");
        }
    }
}
