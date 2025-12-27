use serde_json::Value;

use crate::component::process_components;
use crate::layout::process_layout;
use crate::variables::process_variables;
use crate::BlazeTemplate;

// composes many parsers together to buildup template
pub fn build_template(ctx: &BlazeTemplate, input: &str, data: &Value) -> Result<String, String> {
    let input = process_components(ctx, input, data)?;
    let input = process_layout(ctx, &input, data)?;
    let input = process_variables(&input, data)?;
    Ok(input)
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

    #[test]
    fn it_transforms_simple_layout() {
        let ctx = setup_template_engine();
        let data = json!({"greeting": "Hello", "head": {"name": "Home"}});
        let page_tmpl = "<Layout path='layouts/vars.html'>@greeting</Layout>";
        let result = build_template(&ctx, page_tmpl, &data).unwrap();
        assert_eq!(result, "Header Home Hello Footer\n");
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
}
