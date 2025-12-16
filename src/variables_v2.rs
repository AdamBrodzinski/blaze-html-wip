use serde_json::Value;

use crate::data::get_json_value;

/// replaces any variables with the syntax "name: @name" with the value from Json({"name": "Jane"})
/// with the result "name: Jane". Nested object values can be used with "@person.name.first"
pub fn replace_variables(template: &str, data: &serde_json::Value) -> String {
    // use nom to parse out @variable name
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_two_variables() {
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name, age: @age";

        let result = replace_variables(tmpl, &data);
        assert_eq!(result, "name: Jane, age: 45");
    }

    // #[test]
    // fn it_serializes_booleans() {
    //     let tmpl = "state: @is_open & is_on: false";
    //     let data = json!({"is_open": true, "is_on": false});
    //
    //     let result = replace_variables(tmpl, &data);
    //     assert_eq!(result, "state: true & is_on: false");
    // }
    //
    // #[test]
    // fn it_serializes_number() {
    //     let tmpl = "@a, @b, @c, @d";
    //     let data = json!({"a": 1, "b": 2.0, "c": -3, "d": 0.44});
    //
    //     let result = replace_variables(tmpl, &data);
    //
    //     assert_eq!(result, "1, 2.0, -3, 0.44");
    // }
    //
    // #[test]
    // fn it_serializes_nul_as_empty_str() {
    //     let tmpl = "name: @name";
    //     let data = json!({"name": null});
    //
    //     let result = replace_variables(tmpl, &data);
    //
    //     assert_eq!(result, "name: ");
    // }
    //
    // #[test]
    // fn it_noops_when_data_key_is_not_found() {
    //     let tmpl = "name: @name";
    //     let data = json!({"never": "matches"});
    //
    //     let result = replace_variables(tmpl, &data);
    //
    //     assert_eq!(result, "name: @name");
    // }
    //
    // #[test]
    // fn does_not_transform_emails() {
    //     let tmpl = "foo@bar baz";
    //     let data = json!({ "bar": "Jane" });
    //     let result = replace_variables(tmpl, &data);
    //     assert_eq!(result, "foo@bar baz");
    // }
    //
    // // ----------- nested fields -----------
    //
    // #[test]
    // fn it_returns_nested_fields() {
    //     let tmpl = "name: @person.name";
    //     let data = json!({ "person": { "name": "Jane" } });
    //     let result = replace_variables(tmpl, &data);
    //     assert_eq!(result, "name: Jane");
    // }
    //
    // #[test]
    // fn it_returns_deeply_nested_fields() {
    //     let tmpl = "name: @a.b.c";
    //     let data = json!({ "a": { "b": {"c": "foo"} } });
    //     let result = replace_variables(tmpl, &data);
    //     assert_eq!(result, "name: foo");
    // }
}
