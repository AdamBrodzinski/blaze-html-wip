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

use serde_json::Value;

pub fn render_template_str(template: &str, data: &serde_json::Value) -> String {
    let template = replace_variables(template, data);
    println!("{}", &template);
    template
}

fn replace_variables(template: &str, data: &serde_json::Value) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut last_char = ' ';

    // iterate over the template once, and buildup a new output (String)
    while let Some(c) = chars.next() {
        if c == '@' && !last_char.is_ascii_alphanumeric() {
            // parse the @foo to get the json key name "foo"
            let mut key = String::with_capacity(10);
            while let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_alphanumeric() || next_c == '_' || next_c == '.' {
                    key.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // String type to satisfy match arms
            let orig_var = || format!("@{}", key);

            if let Some(json_value) = get_json_value(data, &key) {
                let value = match json_value {
                    Value::String(x) => x.to_owned(),
                    Value::Bool(x) => x.to_string(),
                    Value::Number(x) => x.to_string(),
                    Value::Null => String::new(),
                    // trying to access the entire object and using that as a template
                    // value will not work, render noop instead
                    Value::Object(_) => orig_var(),
                    // todo
                    Value::Array(_) => orig_var(),
                };
                result.push_str(&value)
            }
            // if data was not found with key, retain the original "@foo"
            else {
                result.push_str(orig_var().as_str());
            }
        }
        // normal character, add it to the result string
        else {
            result.push(c);
        }
        last_char = c;
    }

    result
}

/// extracts the value out of a json data object. it is assumed that the
/// json data is of type Object or it will return None, resulting in a noop
fn get_json_value<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    // current starts out as the json object
    let mut current = data;

    for segment in key.split('.') {
        match current {
            // check if current is an object
            Value::Object(map) => {
                // current for the last segment should be a non object value
                current = map.get(segment)?;
            }
            // if current is not an object return early
            // and don't change anything (noop)
            _ => return None,
        }
    }

    Some(current)
}

#[cfg(test)]
mod replace_variables_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_two_variables() {
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name, age: @age";

        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "name: Jane, age: 45");
    }

    #[test]
    fn it_serializes_booleans() {
        let tmpl = "state: @is_open & is_on: false";
        let data = json!({"is_open": true, "is_on": false});

        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "state: true & is_on: false");
    }

    #[test]
    fn it_serializes_number() {
        let tmpl = "@a, @b, @c, @d";
        let data = json!({"a": 1, "b": 2.0, "c": -3, "d": 0.44});

        let result = replace_variables(tmpl, &data);

        assert_eq!(result, "1, 2.0, -3, 0.44");
    }

    #[test]
    fn it_serializes_nul_as_empty_str() {
        let tmpl = "name: @name";
        let data = json!({"name": null});

        let result = replace_variables(tmpl, &data);

        assert_eq!(result, "name: ");
    }

    #[test]
    fn it_noops_when_data_key_is_not_found() {
        let tmpl = "name: @name";
        let data = json!({"never": "matches"});

        let result = replace_variables(tmpl, &data);

        assert_eq!(result, "name: @name");
    }

    #[test]
    fn does_not_transform_emails() {
        let tmpl = "foo@bar baz";
        let data = json!({ "bar": "Jane" });
        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "foo@bar baz");
    }

    // ----------- nested fields -----------

    #[test]
    fn it_returns_nested_fields() {
        let tmpl = "name: @person.name";
        let data = json!({ "person": { "name": "Jane" } });
        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "name: Jane");
    }

    #[test]
    fn it_returns_deeply_nested_fields() {
        let tmpl = "name: @a.b.c";
        let data = json!({ "a": { "b": {"c": "foo"} } });
        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "name: foo");
    }
}

#[cfg(test)]
mod render_template_str_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removes_comments_test() {
        let tmpl = "<!-- comment --><div>Hello</div>";
        let data = json!({ "foo": 1 });

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<div>Hello</div>");
    }
}
