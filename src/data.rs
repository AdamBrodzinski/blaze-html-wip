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
