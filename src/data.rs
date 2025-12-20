use serde_json::Value;

/// extracts the value out of a json data object. it is assumed that the
/// json data is of type Object or it will return None, resulting in a noop
pub fn get_json_value<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
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

/// extracts the value out of a json data object. it is assumed that the
/// json data is of type Object or it will return an error
pub fn get_json_value_v2<'a>(data: &'a Value, key: &str) -> Result<&'a Value, String> {
    // current starts out as the json object
    let mut current = data;

    for segment in key.split('.') {
        match current {
            Value::Object(map) => {
                // current for the last segment should be a non object value
                current = map
                    .get(segment)
                    .ok_or_else(|| format!("Key '{segment}' not found in object"))?;
            }
            _ => return Err(format!("Cannot access '{segment}' on non-object value")),
        }
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==================== Success Cases ====================

    #[test]
    fn v2_simple_key_access() {
        let data = json!({"name": "Jane"});
        let result = get_json_value_v2(&data, "name");
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn v2_nested_key_access() {
        let data = json!({"person": {"name": "Jane"}});
        let result = get_json_value_v2(&data, "person.name");
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn v2_deeply_nested_key_access() {
        let data = json!({"a": {"b": {"c": {"d": "deep"}}}});
        let result = get_json_value_v2(&data, "a.b.c.d");
        assert_eq!(result.unwrap(), &json!("deep"));
    }

    #[test]
    fn v2_returns_string_value() {
        let data = json!({"name": "Jane"});
        let result = get_json_value_v2(&data, "name");
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn v2_returns_number_value() {
        let data = json!({"age": 25});
        let result = get_json_value_v2(&data, "age");
        assert_eq!(result.unwrap(), &json!(25));
    }

    #[test]
    fn v2_returns_boolean_value() {
        let data = json!({"active": true});
        let result = get_json_value_v2(&data, "active");
        assert_eq!(result.unwrap(), &json!(true));
    }

    #[test]
    fn v2_returns_null_value() {
        let data = json!({"data": null});
        let result = get_json_value_v2(&data, "data");
        assert_eq!(result.unwrap(), &json!(null));
    }

    #[test]
    fn v2_returns_array_value() {
        let data = json!({"items": [1, 2, 3]});
        let result = get_json_value_v2(&data, "items");
        assert_eq!(result.unwrap(), &json!([1, 2, 3]));
    }

    #[test]
    fn v2_returns_object_value() {
        let data = json!({"user": {"name": "Jane", "age": 25}});
        let result = get_json_value_v2(&data, "user");
        assert_eq!(result.unwrap(), &json!({"name": "Jane", "age": 25}));
    }

    // ==================== Error Cases ====================

    #[test]
    fn v2_key_not_found_at_top_level() {
        let data = json!({"name": "Jane"});
        let result = get_json_value_v2(&data, "age");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'age' not found in object");
    }

    #[test]
    fn v2_key_not_found_in_nested_object() {
        let data = json!({"person": {"name": "Jane"}});
        let result = get_json_value_v2(&data, "person.age");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'age' not found in object");
    }

    #[test]
    fn v2_accessing_property_on_string() {
        let data = json!({"name": "Jane"});
        let result = get_json_value_v2(&data, "name.first");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'first' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_property_on_number() {
        let data = json!({"age": 25});
        let result = get_json_value_v2(&data, "age.value");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'value' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_property_on_boolean() {
        let data = json!({"active": true});
        let result = get_json_value_v2(&data, "active.status");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'status' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_property_on_null() {
        let data = json!({"data": null});
        let result = get_json_value_v2(&data, "data.value");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'value' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_property_on_array() {
        let data = json!({"items": [1, 2, 3]});
        let result = get_json_value_v2(&data, "items.length");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'length' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_deeply_nested_missing_key() {
        let data = json!({"a": {"b": {"c": "value"}}});
        let result = get_json_value_v2(&data, "a.b.d");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'd' not found in object");
    }

    #[test]
    fn v2_accessing_key_after_non_object_in_path() {
        let data = json!({"a": {"b": "string"}});
        let result = get_json_value_v2(&data, "a.b.c.d");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot access 'c' on non-object value");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn v2_empty_key_string() {
        let data = json!({"": "empty key"});
        let result = get_json_value_v2(&data, "");
        assert_eq!(result.unwrap(), &json!("empty key"));
    }

    #[test]
    fn v2_key_with_dots_in_name() {
        // This tests that "a.b" is treated as two segments: "a" then "b"
        let data = json!({"a": {"b": "value"}});
        let result = get_json_value_v2(&data, "a.b");
        assert_eq!(result.unwrap(), &json!("value"));
    }

    #[test]
    fn v2_accessing_root_level_non_object() {
        let data = json!("just a string");
        let result = get_json_value_v2(&data, "anything");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'anything' on non-object value"
        );
    }

    #[test]
    fn v2_accessing_root_level_array() {
        let data = json!([1, 2, 3]);
        let result = get_json_value_v2(&data, "0");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot access '0' on non-object value");
    }
}
