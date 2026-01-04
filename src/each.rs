#![allow(unused)]

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_until, take_while1};
use nom::character::complete::{char, space0, space1};
use nom::combinator::{rest, verify};
use nom::{AsBytes, IResult, Parser};
use serde_json::Value;

use crate::data::get_json_value;

type Bytes = [u8];
type Document<'a> = Vec<Node<'a>>;

#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    Text(&'a Bytes),
    Each {
        items_path: &'a str,
        item_name: &'a str,
        index_name: &'a str,
        children: Vec<Node<'a>>,
    },
}

// ==================== Scope Chain ====================

/// A single scope layer in the lookup chain
pub enum ScopeLayer<'a> {
    /// Reference to existing JSON data (e.g., the root context)
    Ref(&'a Value),
    /// An iteration scope with item reference and owned index
    Iteration {
        item_name: &'a str,
        item: &'a Value,
        index_name: &'a str,
        index: usize,
    },
}

/// The scope chain holds a stack of scope layers
/// Lookups search from top (most recent) to bottom (root)
pub struct ScopeChain<'a> {
    layers: Vec<ScopeLayer<'a>>,
}

/// Return type for scope lookups - handles both borrowed and computed values
pub enum ValueRef<'a> {
    Borrowed(&'a Value),
    Index(usize),
}

impl<'a> ScopeChain<'a> {
    pub fn new(root: &'a Value) -> Self {
        Self {
            layers: vec![ScopeLayer::Ref(root)],
        }
    }

    /// Push an iteration scope
    pub fn push_iteration(
        &mut self,
        item_name: &'a str,
        item: &'a Value,
        index_name: &'a str,
        index: usize,
    ) {
        self.layers.push(ScopeLayer::Iteration {
            item_name,
            item,
            index_name,
            index,
        });
    }

    pub fn pop(&mut self) {
        self.layers.pop();
    }

    /// Look up a key in the scope chain (newest to oldest)
    pub fn get(&self, key: &str) -> Result<ValueRef<'a>, String> {
        // Split key into first segment and rest (e.g., "item.name" -> "item", "name")
        let (first_segment, rest) = match key.find('.') {
            Some(pos) => (&key[..pos], Some(&key[pos + 1..])),
            None => (key, None),
        };

        // Search from top to bottom (newest scope first)
        for layer in self.layers.iter().rev() {
            match layer {
                ScopeLayer::Iteration {
                    item_name,
                    item,
                    index_name,
                    index,
                } => {
                    // Check if first segment matches item name
                    if first_segment == *item_name {
                        return match rest {
                            Some(nested_key) => {
                                get_json_value(item, nested_key).map(ValueRef::Borrowed)
                            }
                            None => Ok(ValueRef::Borrowed(item)),
                        };
                    }
                    // Check if it's the index variable (only if no nested path)
                    if first_segment == *index_name && rest.is_none() {
                        return Ok(ValueRef::Index(*index));
                    }
                }
                ScopeLayer::Ref(data) => {
                    // Try to find in this layer's data
                    if let Ok(val) = get_json_value(data, key) {
                        return Ok(ValueRef::Borrowed(val));
                    }
                    // Key not found in this layer, continue to next
                }
            }
        }

        Err(format!("Key '{}' not found in scope chain", key))
    }
}

// ==================== Processing ====================

use crate::variables::process_variables_scoped;

pub fn process_each(input: &str, data: &Value) -> Result<String, String> {
    let (_, nodes) = document(input.as_bytes()).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(input.len());
    let mut scope = ScopeChain::new(data);
    render_nodes(&mut out, &nodes, &mut scope)?;
    String::from_utf8(out).map_err(|e| e.to_string())
}

fn render_nodes<'a>(
    out: &mut Vec<u8>,
    nodes: &[Node<'a>],
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Text(txt) => {
                let text = std::str::from_utf8(txt).map_err(|e| e.to_string())?;
                let processed = process_variables_scoped(text, scope)?;
                out.extend_from_slice(processed.as_bytes());
            }
            Node::Each {
                items_path,
                item_name,
                index_name,
                children,
            } => {
                let array_value = scope.get(items_path)?;
                let array = match array_value {
                    ValueRef::Borrowed(Value::Array(arr)) => arr,
                    _ => return Err(format!("'{}' is not an array", items_path)),
                };

                for (idx, item) in array.iter().enumerate() {
                    scope.push_iteration(item_name, item, index_name, idx + 1);
                    render_nodes(out, children, scope)?;
                    scope.pop();
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
    let (input, (items_path, item_name, index_name)) = open_each(input)?;
    let (input, children) = document(input)?;
    let (input, _) = close_each(input)?;
    Ok((
        input,
        Node::Each {
            items_path,
            item_name,
            index_name,
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

/// Parses `<Each items="@path" as="item_name" idx="index_name">`
/// Returns (items_path, item_name, index_name). `as` and `idx` are optional, defaulting to "item" and "i".
fn open_each(input: &Bytes) -> IResult<&Bytes, (&str, &str, &str)> {
    use nom::error::{Error, ErrorKind};

    // Parse <Each
    let (input, _) = tag(b"<Each " as &Bytes).parse(input)?;

    // Skip optional extra whitespace (already consumed one space in the tag)
    let input = skip_spaces(input);

    // Parse items=
    let (input, _) = tag(b"items=" as &Bytes).parse(input)?;

    // Parse items value
    let (input, items_path) = parse_quoted_value(input)?;
    let items_path = items_path.strip_prefix('@').unwrap_or(items_path);

    let input = skip_spaces(input);

    // Parse optional as="..."
    let (input, item_name) = if input.starts_with(b"as=") {
        let (input, _) = tag(b"as=" as &Bytes).parse(input)?;
        let (input, val) = parse_quoted_value(input)?;
        (skip_spaces(input), val)
    } else {
        (input, "item")
    };

    // Parse optional idx="..."
    let (input, index_name) = if input.starts_with(b"idx=") {
        let (input, _) = tag(b"idx=" as &Bytes).parse(input)?;
        let (input, val) = parse_quoted_value(input)?;
        (skip_spaces(input), val)
    } else {
        (input, "i")
    };

    // Parse closing >
    let (rest, _) = tag(b">" as &Bytes).parse(input)?;

    Ok((rest, (items_path, item_name, index_name)))
}

/// Parses a quoted value (single or double quotes) and returns the inner string
fn parse_quoted_value(input: &Bytes) -> IResult<&Bytes, &str> {
    use nom::error::{Error, ErrorKind};

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

    let value_str = std::str::from_utf8(value)
        .map_err(|_| nom::Err::Error(Error::new(input, ErrorKind::Char)))?;

    Ok((rest, value_str))
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
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items=\"@people\">rest").unwrap();
            assert_eq!(path, "people");
            assert_eq!(item_name, "item");
            assert_eq!(index_name, "i");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_items_single_quotes() {
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items='@users'>rest").unwrap();
            assert_eq!(path, "users");
            assert_eq!(item_name, "item");
            assert_eq!(index_name, "i");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_nested_path() {
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items=\"@data.items\">rest").unwrap();
            assert_eq!(path, "data.items");
            assert_eq!(item_name, "item");
            assert_eq!(index_name, "i");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_as_attribute() {
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items=\"@people\" as=\"person\">rest").unwrap();
            assert_eq!(path, "people");
            assert_eq!(item_name, "person");
            assert_eq!(index_name, "i");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_idx_attribute() {
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items=\"@people\" idx=\"j\">rest").unwrap();
            assert_eq!(path, "people");
            assert_eq!(item_name, "item");
            assert_eq!(index_name, "j");
            assert_eq!(remaining, b"rest");
        }

        #[test]
        fn parses_as_and_idx_attributes() {
            let (remaining, (path, item_name, index_name)) =
                open_each(b"<Each items=\"@people\" as=\"person\" idx=\"j\">rest").unwrap();
            assert_eq!(path, "people");
            assert_eq!(item_name, "person");
            assert_eq!(index_name, "j");
            assert_eq!(remaining, b"rest");
        }
    }

    mod process_as_idx {
        use super::*;

        #[test]
        fn custom_item_name() {
            let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
            let result = process_each(
                r#"<Each items="@people" as="person">@person.name,</Each>"#,
                &data,
            )
            .unwrap();
            assert_eq!(result, "Alice,Bob,");
        }

        #[test]
        fn custom_index_name() {
            let data = json!({"items": [1, 2, 3]});
            let result = process_each(r#"<Each items="@items" idx="j">@j </Each>"#, &data).unwrap();
            assert_eq!(result, "1 2 3 ");
        }

        #[test]
        fn custom_item_and_index() {
            let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
            let result = process_each(
                r#"<ul><Each items="@people" as="person" idx="j"><li>(@j) @person.name</li></Each></ul>"#,
                &data,
            )
            .unwrap();
            assert_eq!(result, "<ul><li>(1) Alice</li><li>(2) Bob</li></ul>");
        }
    }
}
