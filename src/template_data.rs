use serde_json::Value;

/// extracts the value out of a json data object. it is assumed that the
/// json data is of type Object or it will return an error
pub fn get_json_value<'a>(data: &'a Value, segments: &[String]) -> Result<&'a Value, String> {
    if segments.len() == 1 {
        return data
            .get(&segments[0])
            .ok_or_else(|| format!("Key '{}' not found in object", segments[0]));
    }

    let mut current = data;

    for segment in segments {
        match current {
            Value::Object(map) => {
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

    /// Helper to convert a dot-separated key to segments
    fn seg(key: &str) -> Vec<String> {
        key.split('.').map(String::from).collect()
    }

    // ==================== Success Cases ====================

    #[test]
    fn simple_key_access() {
        let data = json!({"name": "Jane"});
        let result = get_json_value(&data, &seg("name"));
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn nested_key_access() {
        let data = json!({"person": {"name": "Jane"}});
        let result = get_json_value(&data, &seg("person.name"));
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn deeply_nested_key_access() {
        let data = json!({"a": {"b": {"c": {"d": "deep"}}}});
        let result = get_json_value(&data, &seg("a.b.c.d"));
        assert_eq!(result.unwrap(), &json!("deep"));
    }

    #[test]
    fn returns_string_value() {
        let data = json!({"name": "Jane"});
        let result = get_json_value(&data, &seg("name"));
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn returns_number_value() {
        let data = json!({"age": 25});
        let result = get_json_value(&data, &seg("age"));
        assert_eq!(result.unwrap(), &json!(25));
    }

    #[test]
    fn returns_boolean_value() {
        let data = json!({"active": true});
        let result = get_json_value(&data, &seg("active"));
        assert_eq!(result.unwrap(), &json!(true));
    }

    #[test]
    fn returns_null_value() {
        let data = json!({"data": null});
        let result = get_json_value(&data, &seg("data"));
        assert_eq!(result.unwrap(), &json!(null));
    }

    #[test]
    fn returns_array_value() {
        let data = json!({"items": [1, 2, 3]});
        let result = get_json_value(&data, &seg("items"));
        assert_eq!(result.unwrap(), &json!([1, 2, 3]));
    }

    #[test]
    fn returns_object_value() {
        let data = json!({"user": {"name": "Jane", "age": 25}});
        let result = get_json_value(&data, &seg("user"));
        assert_eq!(result.unwrap(), &json!({"name": "Jane", "age": 25}));
    }

    // ==================== Error Cases ====================

    #[test]
    fn key_not_found_at_top_level() {
        let data = json!({"name": "Jane"});
        let result = get_json_value(&data, &seg("age"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'age' not found in object");
    }

    #[test]
    fn key_not_found_in_nested_object() {
        let data = json!({"person": {"name": "Jane"}});
        let result = get_json_value(&data, &seg("person.age"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'age' not found in object");
    }

    #[test]
    fn accessing_property_on_string() {
        let data = json!({"name": "Jane"});
        let result = get_json_value(&data, &seg("name.first"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'first' on non-object value"
        );
    }

    #[test]
    fn accessing_property_on_number() {
        let data = json!({"age": 25});
        let result = get_json_value(&data, &seg("age.value"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'value' on non-object value"
        );
    }

    #[test]
    fn accessing_property_on_boolean() {
        let data = json!({"active": true});
        let result = get_json_value(&data, &seg("active.status"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'status' on non-object value"
        );
    }

    #[test]
    fn accessing_property_on_null() {
        let data = json!({"data": null});
        let result = get_json_value(&data, &seg("data.value"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'value' on non-object value"
        );
    }

    #[test]
    fn accessing_property_on_array() {
        let data = json!({"items": [1, 2, 3]});
        let result = get_json_value(&data, &seg("items.length"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot access 'length' on non-object value"
        );
    }

    #[test]
    fn accessing_deeply_nested_missing_key() {
        let data = json!({"a": {"b": {"c": "value"}}});
        let result = get_json_value(&data, &seg("a.b.d"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key 'd' not found in object");
    }

    #[test]
    fn accessing_key_after_non_object_in_path() {
        let data = json!({"a": {"b": "string"}});
        let result = get_json_value(&data, &seg("a.b.c.d"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot access 'c' on non-object value");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn empty_key_string() {
        let data = json!({"": "empty key"});
        let result = get_json_value(&data, &seg(""));
        assert_eq!(result.unwrap(), &json!("empty key"));
    }

    #[test]
    fn nested_key_access_two_segments() {
        let data = json!({"a": {"b": "value"}});
        let result = get_json_value(&data, &seg("a.b"));
        assert_eq!(result.unwrap(), &json!("value"));
    }

    #[test]
    fn accessing_root_level_non_object() {
        let data = json!("just a string");
        let result = get_json_value(&data, &seg("anything"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Key 'anything' not found in object"
        );
    }

    #[test]
    fn accessing_root_level_array() {
        let data = json!([1, 2, 3]);
        let result = get_json_value(&data, &seg("0"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Key '0' not found in object");
    }
}
