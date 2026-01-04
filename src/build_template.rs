use serde_json::Value;

use crate::ast::{parse_template, render_owned, OwnedTemplateNode};
use crate::each::ScopeChain;
use crate::BlazeTemplate;

/// Parse and render a template in a single pass (no caching for inline templates)
#[allow(dead_code)]
pub fn build_template(ctx: &BlazeTemplate, input: &str, data: &Value) -> Result<String, String> {
    let (_, nodes) = parse_template(input).map_err(|e| e.to_string())?;
    let owned = OwnedTemplateNode::vec_from_borrowed(&nodes);
    let mut scope = ScopeChain::new(data);
    render_owned(&owned, ctx, &mut scope)
}

/// tests the integration of template parsers to ensure they work as expected
/// when used together
///
#[allow(clippy::bool_assert_comparison)]
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup_template_engine() -> BlazeTemplate {
        BlazeTemplate::builder()
            .set_root_directory("test_files")
            .build()
    }

    #[test]
    fn builds_static_html() {
        let ctx = setup_template_engine();
        let data = json!(());
        let template = "<div>Hello World</div>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<div>Hello World</div>");
    }

    #[test]
    fn replaces_simple_variables() {
        let ctx = setup_template_engine();
        let data = json!({"greeting": "Hello", "person": {"name": "Jane"}});
        let template = "@greeting @person.name";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "Hello Jane");
    }

    fn setup_with_components() -> BlazeTemplate {
        BlazeTemplate::builder()
            .set_root_directory("test_files")
            .register_component("Card", "components/card.html")
            .register_component("Button", "components/button.html")
            .build()
    }

    #[test]
    fn it_transforms_simple_component() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<Card>Content</Card>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<div class=\"card\">Content</div>\n");
    }

    #[test]
    fn it_transforms_self_closing_component() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<Card />";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<div class=\"card\"></div>\n");
    }

    #[test]
    fn it_transforms_multiple_components() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<Card>A</Card><Card>B</Card>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(
            result,
            "<div class=\"card\">A</div>\n<div class=\"card\">B</div>\n"
        );
    }

    #[test]
    fn it_transforms_different_component_types() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<Card>X</Card><Button>Click</Button>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(
            result,
            "<div class=\"card\">X</div>\n<button>Click</button>\n"
        );
    }

    #[test]
    fn it_ignores_lowercase_tags() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<div>content</div>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<div>content</div>");
    }

    #[test]
    fn it_errors_on_unregistered_component() {
        let ctx = setup_with_components();
        let data = json!(());
        let template = "<Unknown>content</Unknown>";
        let result = build_template(&ctx, template, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[test]
    fn it_processes_components_with_variables() {
        let ctx = setup_with_components();
        let data = json!({"title": "Hello"});
        let template = "<Card>@title</Card>";
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<div class=\"card\">Hello</div>\n");
    }

    #[test]
    fn it_processes_each_with_variables() {
        let ctx = setup_template_engine();
        let data = json!({"people": [{"name": "Alice"}, {"name": "Bob"}]});
        let template = r#"<ul><Each items="@people" as="p"><li>@p.name</li></Each></ul>"#;
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "<ul><li>Alice</li><li>Bob</li></ul>");
    }

    #[test]
    fn it_processes_each_with_index() {
        let ctx = setup_template_engine();
        let data = json!({"items": ["a", "b", "c"]});
        let template = r#"<Each items="@items" as="x" idx="i">(@i) @x </Each>"#;
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(result, "(1) a (2) b (3) c ");
    }

    #[test]
    fn it_processes_nested_each() {
        let ctx = setup_template_engine();
        let data = json!({
            "groups": [
                {"name": "Fruits", "items": [{"name": "Apple"}, {"name": "Banana"}]},
                {"name": "Veggies", "items": [{"name": "Carrot"}]}
            ]
        });
        let template = r#"<Each items="@groups" as="g"><h2>@g.name</h2><ul><Each items="@g.items" as="i"><li>@i.name</li></Each></ul></Each>"#;
        let result = build_template(&ctx, template, &data).unwrap();
        assert_eq!(
            result,
            "<h2>Fruits</h2><ul><li>Apple</li><li>Banana</li></ul><h2>Veggies</h2><ul><li>Carrot</li></ul>"
        );
    }
}
