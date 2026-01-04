//! Unified AST for template parsing
//!
//! This module provides a single-pass parser that recognizes all template constructs:
//! - Variables: @name, @person.email
//! - Escaped: @@
//! - Components: <Card>content</Card>, <Button />
//! - Each loops: <Each items="@items" as="item" idx="i">...</Each>

use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, satisfy, space0};
use nom::combinator::{cut, value};
use nom::sequence::preceded;
use nom::IResult;
use nom::{branch::alt, Parser};
use serde_json::Value;

use crate::each::{ScopeChain, ValueRef};
use crate::BlazeTemplate;

// ==================== AST ====================

#[derive(Debug, PartialEq)]
pub enum TemplateNode<'a> {
    Text(&'a str),
    Variable(&'a str),
    Escaped, // @@
    Component {
        name: &'a str,
        children: Vec<TemplateNode<'a>>,
    },
    Each {
        items_path: &'a str,
        item_name: &'a str,
        idx_name: &'a str,
        children: Vec<TemplateNode<'a>>,
    },
}

// ==================== Parser ====================

/// Parse a complete template into AST nodes
pub fn parse_template(input: &str) -> IResult<&str, Vec<TemplateNode<'_>>> {
    parse_nodes(input, None)
}

/// Parse nodes until we hit a closing tag or end of input
fn parse_nodes<'a>(
    input: &'a str,
    stop_at: Option<&str>,
) -> IResult<&'a str, Vec<TemplateNode<'a>>> {
    let mut nodes = Vec::new();
    let mut remaining = input;

    loop {
        // Check if we've hit the closing tag we're looking for
        if let Some(tag_name) = stop_at {
            if remaining.starts_with("</") {
                let close = format!("</{}>", tag_name);
                if remaining.starts_with(&close) {
                    break;
                }
            }
        }

        // Check if input exhausted
        if remaining.is_empty() {
            break;
        }

        // Try parsers in priority order
        // 1. Each loop (must be before component since <Each starts with uppercase)
        if remaining.starts_with("<Each ") {
            match parse_each(remaining) {
                Ok((rest, node)) => {
                    nodes.push(node);
                    remaining = rest;
                    continue;
                }
                Err(nom::Err::Failure(e)) => return Err(nom::Err::Failure(e)),
                Err(_) => {}
            }
        }

        // 2. Component (<Uppercase>)
        if let Some(b) = remaining.as_bytes().get(1) {
            if remaining.starts_with('<') && b.is_ascii_uppercase() {
                match parse_component(remaining) {
                    Ok((rest, node)) => {
                        nodes.push(node);
                        remaining = rest;
                        continue;
                    }
                    Err(nom::Err::Failure(e)) => return Err(nom::Err::Failure(e)),
                    Err(_) => {}
                }
            }
        }

        // 3. Escaped @@
        if remaining.starts_with("@@") {
            nodes.push(TemplateNode::Escaped);
            remaining = &remaining[2..];
            continue;
        }

        // 4. Variable @name
        if remaining.starts_with('@') {
            if let Ok((rest, node)) = parse_variable(remaining) {
                nodes.push(node);
                remaining = rest;
                continue;
            }
        }

        // 5. Text - consume until next special token
        if let Ok((rest, text)) = parse_text(remaining, stop_at) {
            nodes.push(TemplateNode::Text(text));
            remaining = rest;
            continue;
        }

        // Fallback: consume one character as text
        if !remaining.is_empty() {
            let char_len = remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            nodes.push(TemplateNode::Text(&remaining[..char_len]));
            remaining = &remaining[char_len..];
            continue;
        }

        break;
    }

    Ok((remaining, nodes))
}

/// Parse text until we hit a special token
fn parse_text<'a>(input: &'a str, stop_at: Option<&str>) -> IResult<&'a str, &'a str> {
    let bytes = input.as_bytes();
    let mut end = 0;

    while end < bytes.len() {
        let b = bytes[end];

        // Stop at @ (variable or escape)
        if b == b'@' {
            break;
        }

        // Stop at < if it's a special tag
        if b == b'<' {
            // Check for closing tag we're looking for
            if let Some(tag_name) = stop_at {
                let close = format!("</{}>", tag_name);
                if input[end..].starts_with(&close) {
                    break;
                }
            }
            // Check for <Each
            if input[end..].starts_with("<Each ") {
                break;
            }
            // Check for </Each>
            if input[end..].starts_with("</Each>") {
                break;
            }
            // Check for uppercase component
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

/// Parse @variable.path
fn parse_variable(input: &str) -> IResult<&str, TemplateNode<'_>> {
    let (input, name) = preceded(
        tag("@"),
        take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.'),
    )
    .parse(input)?;
    Ok((input, TemplateNode::Variable(name)))
}

// ==================== Each Parser ====================

/// Parse <Each items="@path" as="item" idx="i">...</Each>
fn parse_each(input: &str) -> IResult<&str, TemplateNode<'_>> {
    let (input, (items_path, item_name, idx_name)) = parse_each_open(input)?;
    let (input, children) = parse_nodes(input, Some("Each"))?;
    let (input, _) = cut(tag("</Each>")).parse(input)?;

    Ok((
        input,
        TemplateNode::Each {
            items_path,
            item_name,
            idx_name,
            children,
        },
    ))
}

/// Parse <Each items="@path" as="item" idx="i">
fn parse_each_open(input: &str) -> IResult<&str, (&str, &str, &str)> {
    let (input, _) = tag("<Each ").parse(input)?;
    let input = skip_spaces(input);

    // Parse items=
    let (input, _) = tag("items=").parse(input)?;
    let (input, items_path) = parse_quoted_value(input)?;
    let items_path = items_path.strip_prefix('@').unwrap_or(items_path);
    let input = skip_spaces(input);

    // Parse optional as="..."
    let (input, item_name) = if input.starts_with("as=") {
        let (input, _) = tag("as=").parse(input)?;
        let (input, val) = parse_quoted_value(input)?;
        (skip_spaces(input), val)
    } else {
        (input, "item")
    };

    // Parse optional idx="..."
    let (input, idx_name) = if input.starts_with("idx=") {
        let (input, _) = tag("idx=").parse(input)?;
        let (input, val) = parse_quoted_value(input)?;
        (skip_spaces(input), val)
    } else {
        (input, "i")
    };

    // Parse closing >
    let (input, _) = tag(">").parse(input)?;

    Ok((input, (items_path, item_name, idx_name)))
}

fn parse_quoted_value(input: &str) -> IResult<&str, &str> {
    let quote_char = input.chars().next().ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Char))
    })?;

    if quote_char != '"' && quote_char != '\'' {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Char,
        )));
    }

    let input = &input[1..]; // skip opening quote
    let close_pos = input.find(quote_char).ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TakeUntil,
        ))
    })?;

    let value = &input[..close_pos];
    let input = &input[close_pos + 1..]; // skip closing quote

    Ok((input, value))
}

fn skip_spaces(input: &str) -> &str {
    input.trim_start_matches(' ')
}

// ==================== Component Parser ====================

/// Parse <Component>...</Component> or <Component />
fn parse_component(input: &str) -> IResult<&str, TemplateNode<'_>> {
    let (input, (tag_name, is_self_closing)) = parse_component_open(input)?;

    if is_self_closing {
        return Ok((
            input,
            TemplateNode::Component {
                name: tag_name,
                children: vec![],
            },
        ));
    }

    let (input, children) = parse_nodes(input, Some(tag_name))?;
    let closing = format!("</{}>", tag_name);
    let (input, _) = cut(tag(closing.as_str())).parse(input)?;

    Ok((
        input,
        TemplateNode::Component {
            name: tag_name,
            children,
        },
    ))
}

/// Parse <ComponentName> or <ComponentName />
fn parse_component_open(input: &str) -> IResult<&str, (&str, bool)> {
    let (input, _) = char('<').parse(input)?;
    let (input, tag_name) = parse_uppercase_tag_name(input)?;
    let (input, _) = space0(input)?;
    let (input, is_self_closing) =
        alt((value(true, tag("/>")), value(false, char('>')))).parse(input)?;
    Ok((input, (tag_name, is_self_closing)))
}

fn parse_uppercase_tag_name(input: &str) -> IResult<&str, &str> {
    nom::combinator::recognize((
        satisfy(|c: char| c.is_ascii_uppercase()),
        take_while1(|c: char| c.is_alphanumeric()),
    ))
    .parse(input)
}

// ==================== Render ====================

/// Render AST nodes to string
pub fn render<'a>(
    nodes: &[TemplateNode<'a>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<String, String> {
    let mut out = String::new();
    render_into(&mut out, nodes, ctx, scope)?;
    Ok(out)
}

fn render_into<'a>(
    out: &mut String,
    nodes: &[TemplateNode<'a>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    for node in nodes {
        match node {
            TemplateNode::Text(t) => out.push_str(t),
            TemplateNode::Escaped => out.push('@'),
            TemplateNode::Variable(path) => {
                render_variable(out, path, scope)?;
            }
            TemplateNode::Component { name, children } => {
                render_component(out, name, children, ctx, scope)?;
            }
            TemplateNode::Each {
                items_path,
                item_name,
                idx_name,
                children,
            } => {
                render_each(out, items_path, item_name, idx_name, children, ctx, scope)?;
            }
        }
    }
    Ok(())
}

fn render_variable(out: &mut String, path: &str, scope: &ScopeChain) -> Result<(), String> {
    match scope.get(path)? {
        ValueRef::Borrowed(json_value) => match json_value {
            Value::String(x) => escape_html_into(x, out),
            Value::Bool(x) => out.push_str(&x.to_string()),
            Value::Number(x) => out.push_str(&x.to_string()),
            Value::Null => return Err(String::from("Cannot render null value")),
            Value::Array(arr) => {
                return Err(format!("Cannot render array to string {:?}", arr));
            }
            Value::Object(obj) => {
                return Err(format!("Cannot render object to string {:?}", obj));
            }
        },
        ValueRef::Index(idx) => {
            out.push_str(&idx.to_string());
        }
    }
    Ok(())
}

fn render_component<'a>(
    out: &mut String,
    name: &str,
    children: &[TemplateNode<'a>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    let path = ctx
        .get_component_path(name)
        .ok_or_else(|| format!("Component '{}' not registered", name))?;

    let template = ctx.read_template(path)?;

    // Render children to string for slot insertion
    let children_html = render(children, ctx, scope)?;
    let with_slot = insert_slot_content(&template, &children_html);

    // Parse and render the component template (may contain more template syntax)
    // Note: template_nodes has a different lifetime than 'a (tied to with_slot)
    // We need a fresh scope for rendering the component template
    let (_, template_nodes) =
        parse_template(&with_slot).map_err(|e| format!("Parse error: {}", e))?;
    render_component_template(out, &template_nodes, ctx, scope)?;

    Ok(())
}

/// Render a component template with its own lifetime
fn render_component_template<'a>(
    out: &mut String,
    nodes: &[TemplateNode<'_>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    for node in nodes {
        match node {
            TemplateNode::Text(t) => out.push_str(t),
            TemplateNode::Escaped => out.push('@'),
            TemplateNode::Variable(path) => {
                render_variable(out, path, scope)?;
            }
            TemplateNode::Component { name, children } => {
                // Recursively render nested components
                let path = ctx
                    .get_component_path(name)
                    .ok_or_else(|| format!("Component '{}' not registered", name))?;
                let template = ctx.read_template(path)?;
                let children_html = render_component_template_to_string(children, ctx, scope)?;
                let with_slot = insert_slot_content(&template, &children_html);
                let (_, template_nodes) =
                    parse_template(&with_slot).map_err(|e| format!("Parse error: {}", e))?;
                render_component_template(out, &template_nodes, ctx, scope)?;
            }
            TemplateNode::Each {
                items_path,
                item_name,
                idx_name,
                children,
            } => {
                // For Each in component templates, we can't push to the scope
                // because the item_name/idx_name lifetimes don't match.
                // We need to handle this differently - use owned strings in a separate scope.
                render_each_in_component(
                    out, items_path, item_name, idx_name, children, ctx, scope,
                )?;
            }
        }
    }
    Ok(())
}

fn render_component_template_to_string<'a>(
    nodes: &[TemplateNode<'_>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<String, String> {
    let mut out = String::new();
    render_component_template(&mut out, nodes, ctx, scope)?;
    Ok(out)
}

fn render_each_in_component<'a>(
    out: &mut String,
    items_path: &str,
    item_name: &str,
    idx_name: &str,
    children: &[TemplateNode<'_>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    let array_value = scope.get(items_path)?;
    let array = match array_value {
        ValueRef::Borrowed(Value::Array(arr)) => arr,
        _ => return Err(format!("'{}' is not an array", items_path)),
    };

    // We need to store item_name and idx_name with owned storage
    // since they come from the component template (different lifetime)
    let item_name_owned: String = item_name.to_string();
    let idx_name_owned: String = idx_name.to_string();

    for (idx, item) in array.iter().enumerate() {
        // Create a fresh scope for each iteration that owns the names
        scope.push_iteration_owned(&item_name_owned, item, &idx_name_owned, idx + 1);
        render_component_template(out, children, ctx, scope)?;
        scope.pop();
    }

    Ok(())
}

fn render_each<'a>(
    out: &mut String,
    items_path: &str,
    item_name: &'a str,
    idx_name: &'a str,
    children: &[TemplateNode<'a>],
    ctx: &BlazeTemplate,
    scope: &mut ScopeChain<'a>,
) -> Result<(), String> {
    let array_value = scope.get(items_path)?;
    let array = match array_value {
        ValueRef::Borrowed(Value::Array(arr)) => arr,
        _ => return Err(format!("'{}' is not an array", items_path)),
    };

    for (idx, item) in array.iter().enumerate() {
        scope.push_iteration(item_name, item, idx_name, idx + 1);
        render_into(out, children, ctx, scope)?;
        scope.pop();
    }

    Ok(())
}

/// Insert content into slot placeholder
fn insert_slot_content(template: &str, content: &str) -> String {
    // Find <slot /> pattern
    let slot_patterns = ["<slot/>", "<slot />", "< slot />", "<  slot   />"];

    for pattern in slot_patterns {
        if let Some(pos) = template.find(pattern) {
            let mut result = String::with_capacity(template.len() + content.len());
            result.push_str(&template[..pos]);
            result.push_str(content);
            result.push_str(&template[pos + pattern.len()..]);
            return result;
        }
    }

    // More flexible: look for <slot with any spacing and />
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let start = i;
            i += 1;
            // Skip whitespace
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            // Check for "slot"
            if i + 4 <= bytes.len() && &bytes[i..i + 4] == b"slot" {
                i += 4;
                // Skip whitespace
                while i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
                // Check for />
                if i + 2 <= bytes.len() && &bytes[i..i + 2] == b"/>" {
                    let end = i + 2;
                    let mut result = String::with_capacity(template.len() + content.len());
                    result.push_str(&template[..start]);
                    result.push_str(content);
                    result.push_str(&template[end..]);
                    return result;
                }
            }
        }
        i += 1;
    }

    // No slot found, return template as-is
    template.to_string()
}

/// Escape HTML special characters
fn escape_html_into(s: &str, output: &mut String) {
    if !s
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\''))
    {
        output.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            _ => output.push(c),
        }
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    mod parser {
        use super::*;

        #[test]
        fn parses_plain_text() {
            let (rest, nodes) = parse_template("hello world").unwrap();
            assert_eq!(rest, "");
            assert_eq!(nodes, vec![TemplateNode::Text("hello world")]);
        }

        #[test]
        fn parses_variable() {
            let (rest, nodes) = parse_template("@name").unwrap();
            assert_eq!(rest, "");
            assert_eq!(nodes, vec![TemplateNode::Variable("name")]);
        }

        #[test]
        fn parses_nested_variable() {
            let (rest, nodes) = parse_template("@person.name").unwrap();
            assert_eq!(rest, "");
            assert_eq!(nodes, vec![TemplateNode::Variable("person.name")]);
        }

        #[test]
        fn parses_escaped() {
            let (rest, nodes) = parse_template("@@").unwrap();
            assert_eq!(rest, "");
            assert_eq!(nodes, vec![TemplateNode::Escaped]);
        }

        #[test]
        fn parses_text_and_variable() {
            let (rest, nodes) = parse_template("Hello @name!").unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![
                    TemplateNode::Text("Hello "),
                    TemplateNode::Variable("name"),
                    TemplateNode::Text("!"),
                ]
            );
        }

        #[test]
        fn parses_self_closing_component() {
            let (rest, nodes) = parse_template("<Card />").unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Component {
                    name: "Card",
                    children: vec![],
                }]
            );
        }

        #[test]
        fn parses_component_with_content() {
            let (rest, nodes) = parse_template("<Card>content</Card>").unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Component {
                    name: "Card",
                    children: vec![TemplateNode::Text("content")],
                }]
            );
        }

        #[test]
        fn parses_nested_components() {
            let (rest, nodes) = parse_template("<Outer><Inner>x</Inner></Outer>").unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Component {
                    name: "Outer",
                    children: vec![TemplateNode::Component {
                        name: "Inner",
                        children: vec![TemplateNode::Text("x")],
                    }],
                }]
            );
        }

        #[test]
        fn parses_each_basic() {
            let (rest, nodes) = parse_template(r#"<Each items="@items">@item</Each>"#).unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Each {
                    items_path: "items",
                    item_name: "item",
                    idx_name: "i",
                    children: vec![TemplateNode::Variable("item")],
                }]
            );
        }

        #[test]
        fn parses_each_with_as() {
            let (rest, nodes) =
                parse_template(r#"<Each items="@people" as="person">@person.name</Each>"#).unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Each {
                    items_path: "people",
                    item_name: "person",
                    idx_name: "i",
                    children: vec![TemplateNode::Variable("person.name")],
                }]
            );
        }

        #[test]
        fn parses_each_with_idx() {
            let (rest, nodes) =
                parse_template(r#"<Each items="@items" idx="j">@j</Each>"#).unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![TemplateNode::Each {
                    items_path: "items",
                    item_name: "item",
                    idx_name: "j",
                    children: vec![TemplateNode::Variable("j")],
                }]
            );
        }

        #[test]
        fn parses_mixed_content() {
            let (rest, nodes) =
                parse_template(r#"<h1>@title</h1><Each items="@items">@item</Each>"#).unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![
                    TemplateNode::Text("<h1>"),
                    TemplateNode::Variable("title"),
                    TemplateNode::Text("</h1>"),
                    TemplateNode::Each {
                        items_path: "items",
                        item_name: "item",
                        idx_name: "i",
                        children: vec![TemplateNode::Variable("item")],
                    },
                ]
            );
        }

        #[test]
        fn lowercase_tags_are_text() {
            let (rest, nodes) = parse_template("<div>content</div>").unwrap();
            assert_eq!(rest, "");
            assert_eq!(nodes, vec![TemplateNode::Text("<div>content</div>")]);
        }

        #[test]
        fn email_escape() {
            let (rest, nodes) = parse_template("email@@example.com").unwrap();
            assert_eq!(rest, "");
            assert_eq!(
                nodes,
                vec![
                    TemplateNode::Text("email"),
                    TemplateNode::Escaped,
                    TemplateNode::Text("example.com"),
                ]
            );
        }
    }

    mod slot {
        use super::*;

        #[test]
        fn inserts_into_slot() {
            let result = insert_slot_content("before <slot /> after", "CONTENT");
            assert_eq!(result, "before CONTENT after");
        }

        #[test]
        fn handles_no_slot() {
            let result = insert_slot_content("no slot here", "CONTENT");
            assert_eq!(result, "no slot here");
        }

        #[test]
        fn handles_compact_slot() {
            let result = insert_slot_content("a<slot/>b", "X");
            assert_eq!(result, "aXb");
        }
    }
}
