use regex::Regex;
use serde_json::Value;

pub fn render_template_str(tmpl: &str, data: serde_json::Value) -> String {
    let reg = Regex::new(r"@(\w+)").expect("Invalid regex");

    // for each template @var_name, look to see if the hashmap has the key "var_name"
    // and replace with the hashmap value
    reg.replace_all(tmpl, |caps: &regex::Captures| {
        let key_name = caps.get(1).expect("expected 1 capture").as_str();
        let entire_match = caps.get(0).unwrap().as_str();

        match data.get(key_name) {
            Some(Value::String(x)) => x.to_owned(),
            Some(Value::Bool(x)) => x.to_string(),
            Some(Value::Number(x)) => x.to_string(),
            Some(Value::Null) => String::from(""),
            Some(Value::Object(_)) => entire_match.to_string(),
            // fallback to no change
            Some(Value::Array(_)) => entire_match.to_string(),
            None => entire_match.to_string(),
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_string_variable() {
        let tmpl = "name: @name";
        let data = json!({"name": "Jane"});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: Jane");
    }

    #[test]
    fn it_replaces_two_variables() {
        let tmpl = "name: @name, age: @age";
        let data = json!({"name": "Jane", "age": "45"});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: Jane, age: 45");
    }

    #[test]
    fn it_serializes_booleans() {
        let tmpl = "state: @is_open & is_on: false";
        let data = json!({"is_open": true, "is_on": false});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "state: true & is_on: false");
    }

    #[test]
    fn it_serializes_number() {
        let tmpl = "@a, @b, @c, @d";
        let data = json!({"a": 1, "b": 2.0, "c": -3, "d": 0.44});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "1, 2.0, -3, 0.44");
    }

    #[test]
    fn it_serializes_nul_as_empty_str() {
        let tmpl = "name: @name";
        let data = json!({"name": null});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: ");
    }

    #[test]
    fn it_noops_when_data_key_is_not_found() {
        let tmpl = "name: @name";
        let data = json!({"never": "matches"});

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: @name");
    }
}
