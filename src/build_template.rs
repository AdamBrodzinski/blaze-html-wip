use serde_json::Value;

use crate::variables::process_variables;
use crate::BlazeTemplate;

// composes many parsers together to buildup template
pub fn build_template(_ctx: &BlazeTemplate, input: &str, data: &Value) -> Result<String, String> {
    let input = process_variables(input, data)?;
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
        BlazeTemplate::new().set_root_directory("test_files")
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
}
