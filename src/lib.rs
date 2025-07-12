//! # Blaze HTML
//!
//! This crate provides another syntax for html templating that works with any
//! HTML formatter/parser. The syntax is similar to web components and tries to
//!
//! ## Features
//! - Variable replacement with context
//!
//! ## Todo
//! - <each> block
//! - <component name="foo" path="/src/templates/foo.html"> include html snippets
//!
// ## Example
// ```rust
// use my_crate::do_foo;
// do_foo();
// ```

use lol_html::html_content::{Comment, ContentType, Element};
use lol_html::{doc_comments, element, rewrite_str, RewriteStrSettings};
use std::fs;

mod variables;
use variables::{get_json_value, replace_variables};

pub fn render_template_str(template: &str, data: &serde_json::Value) -> String {
    let template = rewrite_html(template, data);
    replace_variables(&template, data)
}

fn rewrite_html(template: &str, data: &serde_json::Value) -> String {
    let settings = RewriteStrSettings {
        element_content_handlers: vec![
            element!("component", |el| { handle_component_elements(el, data) }),
            element!("each", |el| { handle_each_elements(el, data) }),
        ],
        document_content_handlers: vec![doc_comments!(remove_html_comments)],
        ..RewriteStrSettings::new()
    };
    // TODO: handle errors instead of falling back to empty template
    rewrite_str(template, settings).unwrap_or_else(|_| template.to_string())
}

fn handle_component_elements(
    el: &mut Element,
    data: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        Some(props_attr) => get_json_value(data, &props_attr)
            .unwrap_or_else(|| panic!("Could not find component props data with key {props_attr}")),
        None => data,
    };
    let processed_contents = replace_variables(contents.trim_end(), conditional_data);

    el.replace(&processed_contents, ContentType::Html);
    Ok(())
}

fn handle_each_elements(
    el: &mut Element,
    data: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let inner_content = "Hello";

    // Get the items attribute to know which array to look at
    let items_key = el.get_attribute("items").unwrap_or("items".to_string());

    // Get the array from data and determine its length
    let array_length = if let Some(array_value) = get_json_value(data, &items_key) {
        if let Some(array) = array_value.as_array() {
            array.len()
        } else {
            0
        }
    } else {
        0
    };

    // Repeat the inner content based on array length
    let repeated_content = inner_content.repeat(array_length);

    el.replace(&repeated_content, ContentType::Html);
    Ok(())
}

fn remove_html_comments(
    comment: &mut Comment,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    comment.remove();
    Ok(())
}

#[cfg(test)]
mod render_template_str_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removes_comments_test() {
        let tmpl = "<!-- comment --><div>Hello</div>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<div>Hello</div>");
    }

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

    #[test]
    fn each_block_basic_two_items_test() {
        let tmpl = "List: <each items='list'>Hello</each>";
        let data_1 = json!({"list": ["a", "b"]});

        let result1 = render_template_str(tmpl, &data_1);
        assert_eq!(result1, "List: HelloHello");
    }

    #[test]
    fn each_block_basic_three_items_test() {
        let tmpl = "List: <each items='list'>Hello</each>";
        let data_2 = json!({"list": ["a", "b", "c"]});

        let result2 = render_template_str(tmpl, &data_2);
        assert_eq!(result2, "List: HelloHelloHello");
    }
}
