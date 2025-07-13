//! # Blaze HTML
//!
//! This crate provides another syntax for html templating that works with any
//! HTML formatter/parser. The syntax is similar to web components and tries to
//!
//! ## Features
//! - Variable replacement with context
//!
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
    // println!("{template}");
    let template_w_each = rewrite_each_tags(template);
    // println!("{template_w_each}");
    let template = rewrite_html(&template_w_each, data);
    // println!("{template}");
    replace_variables(&template, data)
}

fn rewrite_each_tags(template: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    'outer: while !remaining.is_empty() {
        // Find the next <each tag
        if let Some(start_pos) = remaining.find("<each") {
            // Add everything before the tag
            result.push_str(&remaining[..start_pos]);

            // Check if this is a valid tag (followed by > or whitespace)
            let after_tag = &remaining[start_pos + 5..];
            if !after_tag.is_empty() {
                let first_char = after_tag.chars().next().unwrap();
                if first_char == '>' || first_char.is_whitespace() {
                    // Find the end of the opening tag, handling attributes with quotes
                    let mut tag_end_pos = 0;
                    let mut in_quotes = false;
                    let mut quote_char = '\0';

                    for (i, ch) in after_tag.char_indices() {
                        if !in_quotes && ch == '>' {
                            tag_end_pos = i;
                            break;
                        } else if ch == '"' || ch == '\'' {
                            if !in_quotes {
                                in_quotes = true;
                                quote_char = ch;
                            } else if ch == quote_char {
                                in_quotes = false;
                            }
                        }
                    }

                    if tag_end_pos > 0 || (after_tag.chars().next() == Some('>')) {
                        let pos_after_open = start_pos + 5 + tag_end_pos + 1;

                        // Now find the matching closing tag, handling nesting
                        let mut depth = 1;
                        let mut search_pos = pos_after_open;

                        while depth > 0 && search_pos < remaining.len() {
                            // Look for either <each or </each
                            if search_pos + 5 <= remaining.len()
                                && remaining[search_pos..].starts_with("<each")
                            {
                                // Check if it's a valid opening tag
                                if search_pos + 5 < remaining.len() {
                                    let next_char = remaining.as_bytes()[search_pos + 5];
                                    if next_char == b'>' || next_char.is_ascii_whitespace() {
                                        depth += 1;
                                        search_pos += 5;
                                        continue;
                                    }
                                }
                            } else if search_pos + 7 <= remaining.len()
                                && remaining[search_pos..].starts_with("</each>")
                            {
                                depth -= 1;
                                if depth == 0 {
                                    // Found matching closing tag
                                    let content = &remaining[pos_after_open..search_pos];
                                    result.push_str(content);
                                    remaining = &remaining[search_pos + 7..];
                                    continue 'outer;
                                }
                                search_pos += 7;
                                continue;
                            }
                            search_pos += 1;
                        }

                        // If we get here, no matching closing tag was found
                        result.push_str(&remaining[start_pos..]);
                        break;
                    } else {
                        // No closing > for opening tag
                        result.push_str(&remaining[start_pos..]);
                        break;
                    }
                } else {
                    // Not a valid <each tag (e.g., <eachother>)
                    result.push_str(&remaining[start_pos..start_pos + 5]);
                    remaining = &remaining[start_pos + 5..];
                }
            } else {
                // End of string after <each
                result.push_str(&remaining[start_pos..]);
                break;
            }
        } else {
            // No more <each tags
            result.push_str(remaining);
            break;
        }
    }

    result
}

fn rewrite_html(template: &str, data: &serde_json::Value) -> String {
    let settings = RewriteStrSettings {
        element_content_handlers: vec![element!("component", |el| {
            handle_component_elements(el, data)
        })],
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
    fn basic_each_replacement() {
        let tmpl = "List: <each anything='foo'>Hello</each>";
        let data_1 = json!(()); // no data needed yet

        let result1 = render_template_str(tmpl, &data_1);
        assert_eq!(result1, "List: Hello");
    }

    #[test]
    fn nested_each_tags() {
        let tmpl = "<each>outer <each>inner</each> more</each>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "outer <each>inner</each> more");
    }

    #[test]
    fn multiple_each_siblings() {
        let tmpl = "<each>First</each> middle <each>Second</each>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "First middle Second");
    }

    #[test]
    fn deeply_nested_each() {
        let tmpl = "<each>1 <each>2 <each>3</each> 2end</each> 1end</each>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "1 <each>2 <each>3</each> 2end</each> 1end");
    }

    #[test]
    fn each_with_attributes() {
        let tmpl = r#"<each class="test" id="1">Content</each>"#;
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "Content");
    }

    #[test]
    fn each_with_quoted_gt() {
        let tmpl = r#"<each attr="value > test">Content</each>"#;
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "Content");
    }

    #[test]
    fn not_each_tag() {
        let tmpl = "<eachother>Content</eachother>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<eachother>Content</eachother>");
    }

    #[test]
    fn unclosed_each_tag() {
        let tmpl = "<each>Content without closing";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<each>Content without closing");
    }

    // ignore these for now, later we will pass data through
    // #[test]
    // fn each_block_basic_two_items_test() {
    //     let tmpl = "List: <each items='list'>Hello</each>";
    //     let data_1 = json!({"list": ["a", "b"]});
    //
    //     let result1 = render_template_str(tmpl, &data_1);
    //     assert_eq!(result1, "List: HelloHello");
    // }
    //
    // #[test]
    // fn each_block_basic_three_items_test() {
    //     let tmpl = "List: <each items='list'>Hello</each>";
    //     let data_2 = json!({"list": ["a", "b", "c"]});
    //
    //     let result2 = render_template_str(tmpl, &data_2);
    //     assert_eq!(result2, "List: HelloHelloHello");
    // }
}
