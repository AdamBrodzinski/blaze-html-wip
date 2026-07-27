//! Template rendering
//!
//! Renders AST nodes with JSON data to produce the final HTML output.

mod asset;
mod context;

use std::borrow::Cow;

use serde_json::Value;

use crate::ast::{ComponentNode, ConditionMode, EachNode, IfNode, PropValue, TemplateNode};
use crate::error::BlazeError;
use context::RenderContext;

pub(crate) struct ComponentTemplate {
    pub(crate) ast: Vec<TemplateNode>,
    pub(crate) template_len: usize,
}

pub(crate) trait ComponentResolver {
    fn resolve_component(
        &self,
        name: &str,
    ) -> crate::error::Result<std::sync::Arc<ComponentTemplate>>;

    /// Read the raw, unparsed contents of an include file (relative to the template
    /// root) for a `<Include path="..."/>` tag.
    fn resolve_include(&self, path: &str) -> crate::error::Result<std::sync::Arc<String>>;
}

/// The page itself is depth zero, so up to ten nested component renders are allowed.
const MAX_COMPONENT_NESTING_DEPTH: usize = 10;

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
pub(crate) fn render_ast<'a, R: ComponentResolver>(
    ast_nodes: &'a [TemplateNode],
    data: &'a Value,
    template_len: usize,
    resolver: &R,
) -> crate::error::Result<String> {
    let mut ctx = RenderContext::new(data);
    let mut buf = String::with_capacity(template_len);
    render_nodes(ast_nodes, &mut ctx, &mut buf, resolver, 0)?;
    Ok(buf)
}

/// recursively walk down AST and convert each node to text, and then push
/// to a string buffer. complex nodes will accept the buffer and will push
/// inside the node render fn
fn render_nodes<'a, R: ComponentResolver>(
    nodes: &'a [TemplateNode],
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
    resolver: &R,
    component_depth: usize,
) -> crate::error::Result<()> {
    render_nodes_with_slot(nodes, ctx, buf, resolver, component_depth, None)
}

// TODO: rename to `do_render_nodes`, current name suggests this is slot only
fn render_nodes_with_slot<'a, R: ComponentResolver>(
    nodes: &'a [TemplateNode],
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
    resolver: &R,
    component_depth: usize,
    slot: Option<&str>,
) -> crate::error::Result<()> {
    for node in nodes {
        match node {
            TemplateNode::Asset(asset) => asset.write_html(buf)?,
            TemplateNode::Component(component) => {
                render_component(component, ctx, buf, resolver, component_depth)?
            }
            TemplateNode::Each(each) => {
                render_each(each, ctx, buf, resolver, component_depth, slot)?
            }
            TemplateNode::Escaped => buf.push('@'),
            TemplateNode::If(if_node) => {
                render_if(if_node, ctx, buf, resolver, component_depth, slot)?
            }
            TemplateNode::Include(path) => {
                // Splice raw file contents verbatim: no @variable, tag, or HTML escaping.
                let contents = resolver.resolve_include(path)?;
                buf.push_str(&contents);
            }
            TemplateNode::Slot => match slot {
                // Slot content is pre-rendered against the caller's context in
                // render_component, so here it is just spliced in as HTML.
                Some(slot_html) => buf.push_str(slot_html),
                None => {
                    return Err(BlazeError::render(
                        "slot",
                        "slot tag used outside component template",
                    ));
                }
            },
            TemplateNode::Text(x) => buf.push_str(x),
            TemplateNode::Variable(segments) => {
                let json_value = ctx
                    .resolve(segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                render_value(json_value, segments, buf, true)?;
            }
            TemplateNode::VariableRaw(segments) => {
                let json_value = ctx
                    .resolve(segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                render_value(json_value, segments, buf, false)?;
            }
        }
    }
    Ok(())
}

fn render_each<'a, R: ComponentResolver>(
    each: &'a EachNode,
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
    resolver: &R,
    component_depth: usize,
    slot: Option<&str>,
) -> crate::error::Result<()> {
    // `resolve` yields the data lifetime `'a` (not tied to `&ctx`), so we can
    // iterate the array while pushing per-item scopes back into `ctx` without a clone
    let items: &'a [Value] = match ctx
        .resolve(&each.items_path)
        .map_err(|msg| BlazeError::render(each.items_path.join("."), msg))?
    {
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
        render_nodes_with_slot(&each.children, ctx, buf, resolver, component_depth, slot)?;
        ctx.pop_scope();
    }

    Ok(())
}

fn render_if<'a, R: ComponentResolver>(
    if_node: &'a IfNode,
    ctx: &mut RenderContext<'a>,
    buf: &mut String,
    resolver: &R,
    component_depth: usize,
    slot: Option<&str>,
) -> crate::error::Result<()> {
    let condition = match if_node.mode {
        ConditionMode::Exists => ctx.resolve(&if_node.condition_path).is_ok(),
        ConditionMode::Strict | ConditionMode::Truthy => {
            let value = ctx
                .resolve(&if_node.condition_path)
                .map_err(|msg| BlazeError::render(if_node.condition_path.join("."), msg))?;

            match if_node.mode {
                ConditionMode::Strict => match value {
                    Value::Bool(b) => *b,
                    _ => {
                        return Err(BlazeError::render(
                            if_node.condition_path.join("."),
                            "If condition must be a boolean value",
                        ));
                    }
                },
                ConditionMode::Truthy => is_truthy(value),
                ConditionMode::Exists => unreachable!("handled above"),
            }
        }
    };

    let should_render = if if_node.negate {
        !condition
    } else {
        condition
    };

    if should_render {
        render_nodes_with_slot(&if_node.children, ctx, buf, resolver, component_depth, slot)?;
    }

    Ok(())
}

fn render_component<'a, R: ComponentResolver>(
    component: &'a ComponentNode,
    parent_ctx: &mut RenderContext<'a>,
    buf: &mut String,
    resolver: &R,
    component_depth: usize,
) -> crate::error::Result<()> {
    if component_depth >= MAX_COMPONENT_NESTING_DEPTH {
        return Err(BlazeError::render(
            &component.name,
            format!(
                "maximum component nesting depth of {MAX_COMPONENT_NESTING_DEPTH} exceeded while rendering <{}>",
                component.name
            ),
        ));
    }

    let nested_depth = component_depth + 1;
    let template = resolver.resolve_component(&component.name)?;
    // `component_ctx`'s lifetime is inferred at the (shorter) local template Arc
    // lifetime; the longer-lived `&'a` root data and forwarded props coerce down.
    let mut component_ctx = RenderContext::new(parent_ctx.root_data());
    buf.reserve(template.template_len);

    for prop in &component.props {
        match &prop.value {
            // Static props are pre-built `Value`s cached in the AST — borrow them.
            PropValue::Static(value) => component_ctx.push_scope(&prop.name, value),
            // Variable props forward a reference from the parent context. This
            // borrows whatever the parent resolved (root data, an `Each` binding,
            // or another component's static prop) without cloning.
            PropValue::VarPath(segments) => {
                let value = parent_ctx
                    .resolve(segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                component_ctx.push_scope(&prop.name, value);
            }
        }
    }

    // Pre-render the slot content (the component's children) against the parent
    // context. Slot content is evaluated in the caller's scope, which does not change
    // while the component body renders, so rendering it once up front is equivalent to
    // rendering it at each <Slot/>. We produce Some(..) whenever the template has a
    // <Slot/> — even for empty children, which render to "" — so the slot site splices
    // empty content instead of erroring as if used outside a component. None only when
    // the template has no slot, so unused children keep their "not evaluated" behavior.
    let slot_html = if contains_slot(&template.ast) {
        let mut slot_buf = String::new();
        render_nodes_with_slot(
            &component.children,
            parent_ctx,
            &mut slot_buf,
            resolver,
            nested_depth,
            None,
        )?;
        Some(slot_buf)
    } else {
        None
    };

    render_nodes_with_slot(
        &template.ast,
        &mut component_ctx,
        buf,
        resolver,
        nested_depth,
        slot_html.as_deref(),
    )
}

/// Whether the template tree renders a `<Slot/>` in a position that receives this
/// component's slot content. The slot is threaded through `If`/`Each` bodies, but
/// a nested component's children bind to *that* component, so we do not recurse
/// into `Component` nodes here.
fn contains_slot(nodes: &[TemplateNode]) -> bool {
    nodes.iter().any(|node| match node {
        TemplateNode::Slot => true,
        TemplateNode::If(if_node) => contains_slot(&if_node.children),
        TemplateNode::Each(each) => contains_slot(&each.children),
        _ => false,
    })
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn render_value(
    value: &Value,
    segments: &[String],
    buf: &mut String,
    escape: bool,
) -> crate::error::Result<()> {
    match value {
        Value::String(x) => {
            if escape {
                buf.push_str(&html_escape(x));
            } else {
                buf.push_str(x);
            }
        }
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
    #[cfg(feature = "cache-bust")]
    use indoc::indoc;
    use serde_json::json;
    use std::sync::Arc;

    struct NoopResolver;

    impl ComponentResolver for NoopResolver {
        fn resolve_component(&self, name: &str) -> crate::error::Result<Arc<ComponentTemplate>> {
            Err(BlazeError::render(name, "component not registered"))
        }

        fn resolve_include(&self, path: &str) -> crate::error::Result<Arc<String>> {
            Err(BlazeError::render(
                path,
                "include not supported in this resolver",
            ))
        }
    }

    #[cfg(feature = "cache-bust")]
    mod render {
        use super::*;
        use pretty_assertions::assert_eq;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";
        const LOGO_HASH: &str = "1a3cdf7ab74ec558046d1ff5c1546b58";
        const ICON_32_HASH: &str = "08ef72bf778f02a61ac931c19f6131a6";
        const ICON_16_HASH: &str = "5f1c14b22987d18eab40e2ab4dcc94d1";

        // helper to render ast for testing
        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len(), &NoopResolver)
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
                  <script src="/test_files/asset.js?v={}"></script>
                  Name: Jane
                  Email: foo@bar.com
                  <div onclick="alert('inner-quote')"></div>
                  After
                "#},
                JS_HASH
            );
            assert_eq!(html, expected);
        }

        #[test]
        fn renders_image_preload_and_icons() {
            let template = indoc! {r#"
                <Image path="test_files/assets/images/logo.webp" alt="Logo" loading='lazy' />
                <Preload path="test_files/assets/images/logo.webp" as="image" />
                <Icon path="test_files/assets/images/favicon-32x32.png" sizes="32x32" />
                <Icon path="test_files/assets/images/favicon-16x16.png" sizes="16x16" />
            "#};

            let html = render_template(template, &json!({})).unwrap();
            let expected = format!(
                indoc! {r#"
                    <img src="/test_files/assets/images/logo.webp?v={}" alt="Logo" loading='lazy'>
                    <link rel="preload" href="/test_files/assets/images/logo.webp?v={}" as="image">
                    <link rel="icon" href="/test_files/assets/images/favicon-32x32.png?v={}" sizes="32x32">
                    <link rel="icon" href="/test_files/assets/images/favicon-16x16.png?v={}" sizes="16x16">
                "#},
                LOGO_HASH, LOGO_HASH, ICON_32_HASH, ICON_16_HASH,
            );

            assert_eq!(html, expected);
        }
    }

    mod html_escaping {
        use super::*;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len(), &NoopResolver)
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
            render_ast(&ast_nodes, data, page_template.len(), &NoopResolver)
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
            render_ast(&ast_nodes, data, page_template.len(), &NoopResolver)
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
        fn utf8_body_renders() {
            // regression: multibyte chars in a block body must not panic
            let template = r#"<If true="@show">Café 👋 @name 日本語</If>"#;
            let data = json!({"show": true, "name": "Zoé"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Café 👋 Zoé 日本語");
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
        fn exists_renders_when_present() {
            let template = r#"<If exists="@name">Hello @name</If>"#;
            let data = json!({"name": "Ada"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Hello Ada");
        }

        #[test]
        fn exists_renders_for_false_value() {
            let template = r#"<If exists="@active">Active</If>"#;
            let data = json!({"active": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Active");
        }

        #[test]
        fn exists_skips_when_missing() {
            let template = r#"<If exists="@missing">content</If>"#;
            let data = json!({});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn exists_skips_when_nested_missing() {
            let template = r#"<If exists="@user.name">Hello</If>"#;
            let data = json!({"user": {}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
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

    mod if_tag_truthy {
        use super::*;
        use pretty_assertions::assert_eq;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len(), &NoopResolver)
        }

        // truthy tests - should render
        #[test]
        fn truthy_renders_for_true() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn truthy_renders_for_non_empty_string() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": "hello"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn truthy_renders_for_non_zero_number() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": 42});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn truthy_renders_for_negative_number() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": -1});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn truthy_renders_for_non_empty_array() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": [1, 2, 3]});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn truthy_renders_for_non_empty_object() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": {"key": "value"}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        // truthy tests - should NOT render (falsy values)
        #[test]
        fn truthy_skips_for_false() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn truthy_skips_for_null() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": null});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn truthy_skips_for_zero() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": 0});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn truthy_skips_for_empty_string() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": ""});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn truthy_skips_for_empty_array() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": []});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn truthy_skips_for_empty_object() {
            let template = r#"<If truthy="@val">yes</If>"#;
            let data = json!({"val": {}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        // falsy tests (inverse of truthy)
        #[test]
        fn falsy_renders_for_false() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": false});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn falsy_renders_for_null() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": null});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn falsy_renders_for_zero() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": 0});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn falsy_renders_for_empty_string() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": ""});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "yes");
        }

        #[test]
        fn falsy_skips_for_true() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        #[test]
        fn falsy_skips_for_non_empty_string() {
            let template = r#"<If falsy="@val">yes</If>"#;
            let data = json!({"val": "hello"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }

        // scope tests
        #[test]
        fn truthy_inside_each_with_scope() {
            let template =
                r#"<Each items="@items" as="item"><If truthy="@item.name">@item.name </If></Each>"#;
            let data = json!({
                "items": [
                    {"name": "Alice"},
                    {"name": ""},
                    {"name": "Bob"}
                ]
            });
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Alice Bob ");
        }

        #[test]
        fn truthy_with_nested_path() {
            let template = r#"<If truthy="@user.bio">Bio: @user.bio</If>"#;
            let data = json!({"user": {"bio": "Hello world"}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Bio: Hello world");
        }

        #[test]
        fn truthy_with_nested_path_empty() {
            let template = r#"<If truthy="@user.bio">Bio: @user.bio</If>"#;
            let data = json!({"user": {"bio": ""}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "");
        }
    }

    mod components {
        use super::*;
        use std::collections::HashMap;

        struct TestResolver {
            components: HashMap<String, Arc<ComponentTemplate>>,
        }

        impl ComponentResolver for TestResolver {
            fn resolve_component(
                &self,
                name: &str,
            ) -> crate::error::Result<Arc<ComponentTemplate>> {
                self.components
                    .get(name)
                    .cloned()
                    .ok_or_else(|| BlazeError::render(name, "component not registered"))
            }

            fn resolve_include(&self, path: &str) -> crate::error::Result<Arc<String>> {
                Err(BlazeError::render(
                    path,
                    "include not supported in this resolver",
                ))
            }
        }

        #[test]
        fn renders_component_props_and_slot() {
            let component_template = "<div>@first_name <Slot/></div>";
            let component_ast = parse_template_to_ast(component_template).unwrap();
            let mut components = HashMap::new();
            components.insert(
                "Greeting".to_string(),
                Arc::new(ComponentTemplate {
                    ast: component_ast,
                    template_len: component_template.len(),
                }),
            );
            let resolver = TestResolver { components };

            let page_template =
                r#"<Greeting first_name="@person.name">Inner @first_name</Greeting>"#;
            let page_ast = parse_template_to_ast(page_template).unwrap();
            let data = json!({"person": {"name": "Jane"}, "first_name": "Root"});
            let html = render_ast(&page_ast, &data, page_template.len(), &resolver).unwrap();

            assert_eq!(html, "<div>Jane Inner Root</div>");
        }

        #[test]
        fn slot_outside_component_errors() {
            let page_template = "<Slot/>";
            let page_ast = parse_template_to_ast(page_template).unwrap();
            let data = json!({});
            let err = render_ast(&page_ast, &data, page_template.len(), &NoopResolver)
                .unwrap_err()
                .to_string();
            assert!(err.contains("slot tag used outside component"));
        }

        fn resolver_with(components: &[(&str, &str)]) -> TestResolver {
            let mut map = HashMap::new();
            for (name, template) in components {
                let ast = parse_template_to_ast(template).unwrap();
                map.insert(
                    name.to_string(),
                    Arc::new(ComponentTemplate {
                        ast,
                        template_len: template.len(),
                    }),
                );
            }
            TestResolver { components: map }
        }

        fn nested_node(depth: usize) -> Value {
            assert!(depth > 0);
            let mut node = json!({ "value": depth });
            for value in (1..depth).rev() {
                node = json!({ "value": value, "child": node });
            }
            node
        }

        #[test]
        fn finite_recursive_component_allows_ten_levels() {
            let resolver = resolver_with(&[(
                "Node",
                r#"@node.value;<If exists="@node.child"><Node node="@node.child"/></If>"#,
            )]);
            let page = r#"<Node node="@root"/>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "root": nested_node(10) });

            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();

            assert_eq!(html, "1;2;3;4;5;6;7;8;9;10;");
        }

        #[test]
        fn recursive_component_errors_on_the_eleventh_level() {
            let resolver = resolver_with(&[(
                "Node",
                r#"@node.value;<If exists="@node.child"><Node node="@node.child"/></If>"#,
            )]);
            let page = r#"<Node node="@root"/>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "root": nested_node(11) });

            let err = render_ast(&ast, &data, page.len(), &resolver)
                .unwrap_err()
                .to_string();

            assert!(
                err.contains("maximum component nesting depth of 10 exceeded"),
                "unexpected error: {err}"
            );
            assert!(
                err.contains("while rendering <Node>"),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn direct_component_cycle_hits_the_nesting_limit() {
            let resolver = resolver_with(&[("Menu", "<Menu/>")]);
            let page = "<Menu/>";
            let ast = parse_template_to_ast(page).unwrap();

            let err = render_ast(&ast, &json!({}), page.len(), &resolver)
                .unwrap_err()
                .to_string();

            assert!(
                err.contains(
                    "maximum component nesting depth of 10 exceeded while rendering <Menu>"
                ),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn indirect_component_cycle_hits_the_nesting_limit() {
            let resolver = resolver_with(&[
                ("Menu", "<nav><MenuItem/></nav>"),
                ("MenuItem", "<div><Menu/></div>"),
            ]);
            let page = "<Menu/>";
            let ast = parse_template_to_ast(page).unwrap();

            let err = render_ast(&ast, &json!({}), page.len(), &resolver)
                .unwrap_err()
                .to_string();

            assert!(
                err.contains(
                    "maximum component nesting depth of 10 exceeded while rendering <Menu>"
                ),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn slot_components_count_toward_the_nesting_limit() {
            let resolver = resolver_with(&[("Wrap", "<Slot/>")]);
            let page = format!("{}x{}", "<Wrap>".repeat(11), "</Wrap>".repeat(11));
            let ast = parse_template_to_ast(&page).unwrap();

            let err = render_ast(&ast, &json!({}), page.len(), &resolver)
                .unwrap_err()
                .to_string();

            assert!(
                err.contains("maximum component nesting depth of 10 exceeded"),
                "unexpected error: {err}"
            );
        }

        #[test]
        fn slot_content_with_each_mutates_parent_scope() {
            // Slot content is pre-rendered against the parent context; an Each
            // inside it pushes/pops scopes on that parent. Regression for the
            // safe (clone-based) slot + Each rendering.
            let resolver = resolver_with(&[("Wrap", "<section><Slot/></section>")]);
            let page = r#"<Wrap><Each items="@xs" as="x">@x;</Each></Wrap>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "xs": ["a", "b", "c"] });
            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();
            assert_eq!(html, "<section>a;b;c;</section>");
        }

        #[test]
        fn slot_inside_if_with_varpath_prop() {
            // Template uses <Slot/> inside an If (contains_slot must recurse),
            // a VarPath prop (resolved from parent and forwarded into the component
            // scope), and slot content that reads a parent-scope variable.
            let resolver =
                resolver_with(&[("Card", r#"<If true="@show"><h2>@title</h2> <Slot/></If>"#)]);
            let page = r#"<Card show="@flag" title="Hi">body @name</Card>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "flag": true, "name": "Zoé" });
            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();
            assert_eq!(html, "<h2>Hi</h2> body Zoé");
        }

        #[test]
        fn empty_children_with_slot_template_renders_empty() {
            // A component template with a <Slot/> but no children provided renders an
            // empty slot rather than erroring with "slot tag used outside component".
            let resolver = resolver_with(&[("Card", "<div><Slot/></div>")]);
            let page = r#"<Card></Card>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({});
            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();
            assert_eq!(html, "<div></div>");
        }

        #[test]
        fn slot_content_is_pre_rendered_even_when_if_is_untaken() {
            // Slot content is pre-rendered eagerly against the caller's context, so a
            // broken variable in it surfaces an error even when the <Slot/> sits inside
            // an untaken <If> and is never spliced. This is an accepted limitation of
            // eager pre-rendering (errors are surfaced early rather than skipped).
            let resolver = resolver_with(&[("Card", r#"<If true="@show"><Slot/></If>"#)]);
            let page = r#"<Card show="@flag">@missing_var</Card>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "flag": false });
            let err = render_ast(&ast, &data, page.len(), &resolver)
                .unwrap_err()
                .to_string();
            assert!(err.contains("missing_var"), "unexpected error: {err}");
        }

        #[test]
        fn slot_inside_taken_if_renders_content() {
            // When the <If> is taken, the pre-rendered slot content is spliced in,
            // resolved against the caller's scope.
            let resolver = resolver_with(&[("Card", r#"<If true="@show">x<Slot/></If>"#)]);
            let page = r#"<Card show="@flag">@name</Card>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "flag": true, "name": "Zoé" });
            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();
            assert_eq!(html, "xZoé");
        }

        #[test]
        fn slot_inside_empty_each_errors_eagerly() {
            // A <Slot/> reachable through an <Each> is pre-rendered, so broken slot
            // content surfaces an error even when the array is empty.
            let resolver = resolver_with(&[("List", r#"<Each items="@xs" as="x"><Slot/></Each>"#)]);
            let page = r#"<List>@missing_var</List>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "xs": [] });
            let err = render_ast(&ast, &data, page.len(), &resolver)
                .unwrap_err()
                .to_string();
            assert!(err.contains("missing_var"), "unexpected error: {err}");
        }

        #[test]
        fn slot_inside_each_renders_for_every_item() {
            // A <Slot/> inside an <Each> body is spliced for every item.
            let resolver = resolver_with(&[("List", r#"<Each items="@xs" as="x"><Slot/></Each>"#)]);
            let page = r#"<List>@label;</List>"#;
            let ast = parse_template_to_ast(page).unwrap();
            let data = json!({ "xs": [1, 2, 3], "label": "y" });
            let html = render_ast(&ast, &data, page.len(), &resolver).unwrap();
            assert_eq!(html, "y;y;y;");
        }
    }
}
