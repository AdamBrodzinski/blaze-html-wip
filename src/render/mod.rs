//! Template rendering
//!
//! Renders AST nodes with JSON data to produce the final HTML output.

mod asset;
mod context;

use std::borrow::Cow;

use serde_json::Value;

use crate::ast::{EachNode, IfNode, TemplateNode};
use crate::error::BlazeError;
use context::RenderContext;

fn html_escape(s: &str) -> Cow<'_, str> {
    let first_special_idx = s.find(['<', '>', '&', '"', '\'']);
    match first_special_idx {
        None => Cow::Borrowed(s),
        Some(byte_idx) => {
            let mut escaped = String::with_capacity(s.len() + 16);
            escaped.push_str(&s[..byte_idx]); // copy prefix that doesn't need escaping

            for c in s[byte_idx..].chars() {
                match c {
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '&' => escaped.push_str("&amp;"),
                    '"' => escaped.push_str("&quot;"),
                    '\'' => escaped.push_str("&#39;"),
                    _ => escaped.push(c),
                }
            }
            Cow::Owned(escaped)
        }
    }
}

// main entry point for rendering AST nodes
pub fn render_ast<'a>(
    ast_nodes: &'a [TemplateNode],
    data: &'a Value,
    template_len: usize,
) -> crate::error::Result<String> {
    let mut ctx = RenderContext::new(data);
    let mut buf = String::with_capacity(template_len);
    render_nodes(ast_nodes, &mut ctx, &mut buf)?;
    Ok(buf)
}

// separate render_nodes from render_ast for recursion
fn render_nodes<'a>(
    nodes: &'a [TemplateNode],
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
) -> crate::error::Result<()> {
    for node in nodes {
        match node {
            TemplateNode::Asset(asset) => asset.write_html(buf)?,
            TemplateNode::Each(each) => render_each(each, ctx, buf)?,
            TemplateNode::Escaped => buf.push('@'),
            TemplateNode::If(if_node) => render_if(if_node, ctx, buf)?,
            TemplateNode::Text(x) => buf.push_str(x),
            TemplateNode::Variable(segments) => {
                let json_value = ctx
                    .resolve(segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                render_value_escaped(json_value, segments, buf)?;
            }
            TemplateNode::VariableRaw(segments) => {
                let json_value = ctx
                    .resolve(segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                render_value_raw(json_value, segments, buf)?;
            }
        }
    }
    Ok(())
}

fn render_each<'a>(
    each: &'a EachNode,
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
) -> crate::error::Result<()> {
    // find the topmost 'context layer' that matches the key, allows inner scopes to shadow outer
    let collection = ctx
        .resolve(&each.items_path)
        .map_err(|msg| BlazeError::render(each.items_path.join("."), msg))?;

    let items = match collection {
        Value::Array(arr) => arr,
        _ => {
            return Err(BlazeError::render(
                each.items_path.join("."),
                "expected array for Each items",
            ));
        }
    };

    for item in items {
        ctx.push_scope(&each.item_binding, item);
        // ctx is passed in recursively
        render_nodes(&each.children, ctx, buf)?;
        ctx.pop_scope();
    }

    Ok(())
}

fn render_if<'a>(
    if_node: &'a IfNode,
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
) -> crate::error::Result<()> {
    let value = ctx
        .resolve(&if_node.condition_path)
        .map_err(|msg| BlazeError::render(if_node.condition_path.join("."), msg))?;

    let condition = match value {
        Value::Bool(b) => *b,
        _ => {
            return Err(BlazeError::render(
                if_node.condition_path.join("."),
                "If condition must be a boolean value",
            ));
        }
    };

    let should_render = if if_node.negate {
        !condition
    } else {
        condition
    };

    if should_render {
        render_nodes(&if_node.children, ctx, buf)?;
    }

    Ok(())
}

// TODO: combine escaped/raw with an escaped bool flag
fn render_value_escaped(
    value: &Value,
    segments: &[String],
    buf: &mut String,
) -> crate::error::Result<()> {
    match value {
        Value::String(x) => buf.push_str(&html_escape(x)),
        Value::Bool(x) => buf.push_str(&x.to_string()),
        Value::Number(x) => buf.push_str(&x.to_string()),
        Value::Null => buf.push_str("null"),
        Value::Array(_) => {
            return Err(BlazeError::render(
                segments.join("."),
                "cannot render array as string",
            ));
        }
        Value::Object(_) => {
            return Err(BlazeError::render(
                segments.join("."),
                "cannot render object as string",
            ));
        }
    }
    Ok(())
}

fn render_value_raw(
    value: &Value,
    segments: &[String],
    buf: &mut String,
) -> crate::error::Result<()> {
    match value {
        Value::String(x) => buf.push_str(x),
        Value::Bool(x) => buf.push_str(&x.to_string()),
        Value::Number(x) => buf.push_str(&x.to_string()),
        Value::Null => buf.push_str("null"),
        Value::Array(_) => {
            return Err(BlazeError::render(
                segments.join("."),
                "cannot render array as string",
            ));
        }
        Value::Object(_) => {
            return Err(BlazeError::render(
                segments.join("."),
                "cannot render object as string",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_template_to_ast;
    use indoc::indoc;
    use serde_json::json;

    #[cfg(feature = "cache-bust")]
    mod render {
        use super::*;
        use pretty_assertions::assert_eq;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";

        // helper to render ast for testing
        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn render_basic_html() {
            let template = indoc! {r#"
                Before
                <Script path='test_files/asset.js' />
                Name: @person.first_name1
                Email: foo@@bar.com
                <div onclick="alert('inner-quote')"></div>
                After
            "#};
            let data = json!({"person": {"first_name1": "Jane"}});
            let html = render_template(template, &data).unwrap();
            let expected = format!(
                indoc! {r#"
                  Before
                  <script src="test_files/asset.js?{}"></script>
                  Name: Jane
                  Email: foo@bar.com
                  <div onclick="alert('inner-quote')"></div>
                  After
                "#},
                JS_HASH
            );
            assert_eq!(html, expected);
        }
    }

    mod html_escaping {
        use super::*;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn escapes_html_in_variables() {
            let template = "Hello @name!";
            let data = json!({"name": "<script>alert('xss')</script>"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(
                html,
                "Hello &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;!"
            );
        }

        #[test]
        fn escapes_all_special_chars() {
            let template = "@content";
            let data = json!({"content": "<>&\"'"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "&lt;&gt;&amp;&quot;&#39;");
        }

        #[test]
        fn raw_variable_does_not_escape() {
            let template = "Hello @!name!";
            let data = json!({"name": "<b>bold</b>"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Hello <b>bold</b>!");
        }

        #[test]
        fn mixed_escaped_and_raw() {
            let template = "User: @user.name Bio: @!user.bio";
            let data = json!({"user": {"name": "<script>", "bio": "<p>Hello</p>"}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "User: &lt;script&gt; Bio: <p>Hello</p>");
        }

        #[test]
        fn numbers_not_escaped() {
            let template = "Count: @count";
            let data = json!({"count": 42});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Count: 42");
        }

        #[test]
        fn bools_not_escaped() {
            let template = "Active: @active";
            let data = json!({"active": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Active: true");
        }
    }

    mod each_tag {
        use super::*;
        use pretty_assertions::assert_eq;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn simple_iteration() {
            let template = r#"<Each items="@people" as="p">@p.name </Each>"#;
            let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Alice Bob ");
        }

        #[test]
        fn nested_iteration() {
            let template = r#"<Each items="@people" as="p">@p.name: <Each items="@p.skills" as="s">@s </Each></Each>"#;
            let data = json!({
                "people": [
                    {"name": "Alice", "skills": ["Rust", "Go"]},
                    {"name": "Bob", "skills": ["Python"]}
                ]
            });
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Alice: Rust Go Bob: Python ");
        }

        #[test]
        fn outer_scope_accessible() {
            let template = r#"<Each items="@items" as="item">@title: @item </Each>"#;
            let data = json!({"title": "Item", "items": ["A", "B", "C"]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Item: A Item: B Item: C ");
        }

        #[test]
        fn empty_array_renders_nothing() {
            let template = r#"Before<Each items="@items" as="item">@item</Each>After"#;
            let data = json!({"items": []});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "BeforeAfter");
        }

        #[test]
        fn error_on_non_array() {
            let template = r#"<Each items="@name" as="x">@x</Each>"#;
            let data = json!({"name": "not an array"});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("expected array"));
        }

        #[test]
        fn deeply_nested() {
            let template = r#"<Each items="@a" as="x"><Each items="@x" as="y"><Each items="@y" as="z">@z</Each></Each></Each>"#;
            let data = json!({"a": [[["1", "2"], ["3"]], [["4"]]]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "1234");
        }

        #[test]
        fn with_html_content() {
            let template = r#"<ul><Each items="@items" as="item"><li>@item</li></Each></ul>"#;
            let data = json!({"items": ["A", "B"]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "<ul><li>A</li><li>B</li></ul>");
        }

        #[test]
        fn iterating_objects() {
            let template = r#"<Each items="@users" as="u">@u.id:@u.name;</Each>"#;
            let data = json!({
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            });
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "1:Alice;2:Bob;");
        }

        #[test]
        fn html_escaping_in_each() {
            let template = r#"<Each items="@items" as="item">@item</Each>"#;
            let data = json!({"items": ["<script>", "&amp;"]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "&lt;script&gt;&amp;amp;");
        }

        #[test]
        fn raw_variable_in_each() {
            let template = r#"<Each items="@items" as="item">@!item</Each>"#;
            let data = json!({"items": ["<b>bold</b>"]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "<b>bold</b>");
        }
    }

    mod if_tag {
        use super::*;
        use pretty_assertions::assert_eq;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn true_condition_renders_when_true() {
            let template = r#"<If true="@active">Active</If>"#;
            let data = json!({"active": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Active");
        }

        #[test]
        fn true_condition_skips_when_false() {
            let template = r#"<If true="@active">Active</If>"#;
            let data = json!({"active": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn false_condition_renders_when_false() {
            let template = r#"<If false="@active">Inactive</If>"#;
            let data = json!({"active": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Inactive");
        }

        #[test]
        fn false_condition_skips_when_true() {
            let template = r#"<If false="@active">Inactive</If>"#;
            let data = json!({"active": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn nested_path_resolution() {
            let template = r#"<If true="@user.is_admin">Admin</If>"#;
            let data = json!({"user": {"is_admin": true}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Admin");
        }

        #[test]
        fn with_variables_inside() {
            let template = r#"<If true="@show">Hello @name!</If>"#;
            let data = json!({"show": true, "name": "World"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Hello World!");
        }

        #[test]
        fn multiple_if_tags() {
            let template = r#"<If true="@one">One</If><If false="@one">NotOne</If>"#;
            let data = json!({"one": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "One");
        }

        #[test]
        fn nested_if_tags() {
            let template = r#"<If true="@outer"><If true="@inner">Both</If></If>"#;
            let data = json!({"outer": true, "inner": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Both");
        }

        #[test]
        fn nested_if_outer_false() {
            let template = r#"<If true="@outer"><If true="@inner">Both</If></If>"#;
            let data = json!({"outer": false, "inner": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn inside_each_with_scope() {
            let template =
                r#"<Each items="@items" as="item"><If true="@item.active">@item.name </If></Each>"#;
            let data = json!({
                "items": [
                    {"name": "Alice", "active": true},
                    {"name": "Bob", "active": false},
                    {"name": "Charlie", "active": true}
                ]
            });
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Alice Charlie ");
        }

        #[test]
        fn inside_each_accessing_outer_scope() {
            let template =
                r#"<Each items="@items" as="item"><If true="@show_all">@item </If></Each>"#;
            let data = json!({"show_all": true, "items": ["A", "B", "C"]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "A B C ");
        }

        #[test]
        fn error_on_missing_variable() {
            let template = r#"<If true="@missing">content</If>"#;
            let data = json!({});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("missing"));
        }

        #[test]
        fn error_on_non_boolean_string() {
            let template = r#"<If true="@value">content</If>"#;
            let data = json!({"value": "yes"});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("boolean"));
        }

        #[test]
        fn error_on_non_boolean_number() {
            let template = r#"<If true="@value">content</If>"#;
            let data = json!({"value": 1});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("boolean"));
        }

        #[test]
        fn error_on_non_boolean_null() {
            let template = r#"<If true="@value">content</If>"#;
            let data = json!({"value": null});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("boolean"));
        }

        #[test]
        fn error_on_non_boolean_array() {
            let template = r#"<If true="@value">content</If>"#;
            let data = json!({"value": [1, 2, 3]});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("boolean"));
        }

        #[test]
        fn error_on_non_boolean_object() {
            let template = r#"<If true="@value">content</If>"#;
            let data = json!({"value": {"foo": "bar"}});
            let result = render_template(template, &data);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("boolean"));
        }

        #[test]
        fn user_example_from_requirements() {
            // data: { "one_is_active": true, "two_is_active": false }
            let template = r#"<If true="@one_is_active">One Active</If>
<If false="@one_is_active">One Disabled</If>
<If true="@two_is_active">Two Active</If>
<If false="@two_is_active">Two Disabled</If>"#;
            let data = json!({"one_is_active": true, "two_is_active": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "One Active\n\n\nTwo Disabled");
        }
    }
}
