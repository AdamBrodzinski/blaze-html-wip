use crate::data::get_json_value;
use crate::variables::replace_variables;
use lol_html::html_content::ContentType;
use lol_html::{element, rewrite_str, RewriteStrSettings};
use std::fs;

pub fn rewrite_component(template: &str, data: &serde_json::Value) -> String {
    let settings = RewriteStrSettings {
        element_content_handlers: vec![element!("component", |el| {
            let path = el
                .get_attribute("path")
                .expect("Could not find path attr on component");

            let contents = fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Could not read component file at '{path}'"));

            // check if the component has a props variable, if it does, find the props key,
            // extract the JSON data from the template_data by the props key, and set that
            // value as the "data" for the component. the component is effectively rendered
            // in isolation and then the final result replaces the component tag. This can
            // be recursively called for the nested component use case, but the nested template
            // only has access to the data passed in from the parent
            let conditional_data = match el.get_attribute("props") {
                Some(props_attr) => get_json_value(data, &props_attr).unwrap_or_else(|| {
                    panic!("Could not find component props data with key {props_attr}")
                }),
                None => data,
            };
            let processed_contents = replace_variables(contents.trim_end(), conditional_data);

            el.replace(&processed_contents, ContentType::Html);
            Ok(())
        })],
        ..RewriteStrSettings::new()
    };
    // TODO: handle errors instead of falling back to empty template
    rewrite_str(template, settings).unwrap_or_else(|_| template.to_string())
}

#[cfg(test)]
mod tests {
    use crate::render_template_str;
    use serde_json::json;

    #[test]
    fn component_happy_path_test() {
        let tmpl = "Hello <component path='test_files/component.html' />";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "Hello <div>Component</div>");
    }

    #[test]
    fn component_props_data_test() {
        let tmpl = "<component props='person' path='test_files/component-props.html' />";
        let data = json!({"my_global": "Global", "person": {"name": "Jane"}});

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "Props name: Jane, global var: Global");
    }

    #[test]
    #[should_panic(expected = "Could not read component file at 'invalid-path'")]
    fn component_missing_path_test() {
        let tmpl = "Hello <component path='invalid-path' />";
        let data = json!(());

        render_template_str(tmpl, &data);
    }
}
