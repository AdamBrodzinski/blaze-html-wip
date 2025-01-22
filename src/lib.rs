use serde_json::Value;

pub fn render_template_str(template: &str, data: &serde_json::Value) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    // iterate over the template once, and buildup a new output (String)
    while let Some(c) = chars.next() {
        if c == '@' {
            // parse the @foo to get the json key name "foo"
            let mut key = String::with_capacity(10);
            while let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_alphanumeric() || next_c == '_' {
                    key.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            if let Some(json_value) = data.get(&key) {
                let value = match json_value {
                    Value::String(x) => x.to_owned(),
                    Value::Bool(x) => x.to_string(),
                    Value::Number(x) => x.to_string(),
                    Value::Null => String::new(),
                    // fallback to no change
                    Value::Object(_) => format!("@{}", key),
                    Value::Array(_) => format!("@{}", key),
                };
                result.push_str(&value)
            }
            // if data was not found with key, retain the original "@foo"
            else {
                result.push('@');
                result.push_str(&key);
            }
        }
        // normal character, add it to the result string
        else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Instant;

    static NAME_AGE: &str = "name: @name, age: @age";

    #[test]
    fn it_replaces_two_variables() {
        let data = json!({"name": "Jane", "age": "45"});

        let start = Instant::now();

        let result = render_template_str(NAME_AGE, &data);

        let duration = start.elapsed();
        println!("Time: {:?}", duration);

        assert_eq!(result, "name: Jane, age: 45");
    }

    #[test]
    fn it_serializes_booleans() {
        let tmpl = "state: @is_open & is_on: false";
        let data = json!({"is_open": true, "is_on": false});

        let result = render_template_str(tmpl, &data);

        assert_eq!(result, "state: true & is_on: false");
    }

    #[test]
    fn it_serializes_number() {
        let tmpl = "@a, @b, @c, @d";
        let data = json!({"a": 1, "b": 2.0, "c": -3, "d": 0.44});

        let result = render_template_str(tmpl, &data);

        assert_eq!(result, "1, 2.0, -3, 0.44");
    }

    #[test]
    fn it_serializes_nul_as_empty_str() {
        let tmpl = "name: @name";
        let data = json!({"name": null});

        let result = render_template_str(tmpl, &data);

        assert_eq!(result, "name: ");
    }

    #[test]
    fn it_noops_when_data_key_is_not_found() {
        let tmpl = "name: @name";
        let data = json!({"never": "matches"});

        let result = render_template_str(tmpl, &data);

        assert_eq!(result, "name: @name");
    }

    // #[test]
    // fn it_renders_each_blocks() {
    //     let tmpl = r#"hello "world" bar "#; // foo
    //                                         // let data = json!({"never": "matches"});
    //                                         //
    //                                         // let result = render_template_str(tmpl, data);
    //
    //     assert_eq!(tmpl, "name: @name");
    // }
}
