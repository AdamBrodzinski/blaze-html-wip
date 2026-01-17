use serde_json::Value;

/// each layer in the variable scope chain
struct ScopeLayer<'a> {
    name: &'a str,    // <Each items="foo" name
    value: &'a Value, // JSON data
}

/// Render context with scope chain for variable resolution.
/// The scope chain is searched from innermost (last) to outermost (first).
/// The root_data is the fallback for top-level variables.
pub struct RenderContext<'a> {
    root_data: &'a Value,
    scope_stack: Vec<ScopeLayer<'a>>,
}

impl<'a> RenderContext<'a> {
    pub fn new(root_data: &'a Value) -> Self {
        Self {
            root_data,
            scope_stack: Vec::new(),
        }
    }

    pub fn push_scope(&mut self, name: &'a str, value: &'a Value) {
        self.scope_stack.push(ScopeLayer { name, value });
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Resolve a variable path (e.g., ["person", "name"])
    ///
    /// Resolution order:
    /// 1. Check if first segment matches a binding in any scope (innermost first)
    /// 2. If matched, resolve remaining segments from that value
    /// 3. If no match, resolve from root_data
    pub fn resolve(&self, segments: &[String]) -> Result<&'a Value, String> {
        if segments.is_empty() {
            return Err("Empty variable path".to_string());
        }

        let first_segment = &segments[0];

        // search scope stack from innermost to outermost
        for scope in self.scope_stack.iter().rev() {
            // "person" in ["person", "name"] or in ["person"]
            if scope.name == first_segment {
                // segments is ["person"], exact match for "person"
                if segments.len() == 1 {
                    return Ok(scope.value);
                }
                // segments is ["person", "name"], matched "person", pass remaining ["name"] to resolve remaining path
                return resolve_path(scope.value, &segments[1..]);
            }
        }

        // TODO: look into optimization for the non nested scope data case
        // not in any scope - resolve from root data
        resolve_path(self.root_data, segments)
    }
}

/// access nested data {"person": {"name": "Jane"}} with ["person", "name"] == Ok("Jane")
fn resolve_path<'a>(data: &'a Value, segments: &[String]) -> Result<&'a Value, String> {
    // TODO: return error if empty?
    if segments.is_empty() {
        return Ok(data);
    }
    let mut current = data;
    for segment in segments {
        match current {
            Value::Object(map) => {
                current = map
                    .get(segment)
                    .ok_or_else(|| format!("Key '{}' not found in object", segment))?;
            }
            _ => return Err(format!("Cannot access '{}' on non-object value", segment)),
        }
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_from_root() {
        let data = json!({"name": "Jane", "age": 30});
        let ctx = RenderContext::new(&data);

        let result = ctx.resolve(&["name".to_string()]).unwrap();
        assert_eq!(result, &json!("Jane"));
    }

    #[test]
    fn resolve_nested_from_root() {
        let data = json!({"person": {"name": "Jane"}});
        let ctx = RenderContext::new(&data);

        let result = ctx
            .resolve(&["person".to_string(), "name".to_string()])
            .unwrap();
        assert_eq!(result, &json!("Jane"));
    }

    #[test]
    fn resolve_from_scope() {
        let data = json!({"people": [{"name": "Jane"}]});
        let person = &data["people"][0];

        let mut ctx = RenderContext::new(&data);
        ctx.push_scope("person", person);

        // Should resolve from scope
        let result = ctx.resolve(&["person".to_string(), "name".to_string()]);
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn scope_shadows_root() {
        let data = json!({"name": "Root", "person": {"name": "Nested"}});
        let nested = &data["person"];

        let mut ctx = RenderContext::new(&data);
        ctx.push_scope("name", nested);

        // "name" now refers to the scope binding, not root
        let result = ctx.resolve(&["name".to_string()]).unwrap();
        assert_eq!(result, &json!({"name": "Nested"}));
    }

    #[test]
    fn inner_scope_shadows_outer() {
        let data = json!({
            "items": [
                {"value": "outer"},
                {"value": "inner"}
            ]
        });
        let outer = &data["items"][0];
        let inner = &data["items"][1];

        let mut ctx = RenderContext::new(&data);
        ctx.push_scope("item", outer);
        ctx.push_scope("item", inner);

        // Inner scope wins
        let result = ctx
            .resolve(&["item".to_string(), "value".to_string()])
            .unwrap();
        assert_eq!(result, &json!("inner"));

        ctx.pop_scope();

        // Now outer scope is visible
        let result = ctx
            .resolve(&["item".to_string(), "value".to_string()])
            .unwrap();
        assert_eq!(result, &json!("outer"));
    }

    #[test]
    fn outer_scope_accessible_from_inner() {
        let data = json!({
            "page_name": "Test Page",
            "people": [{"name": "Jane"}]
        });
        let person = &data["people"][0];

        let mut ctx = RenderContext::new(&data);
        ctx.push_scope("person", person);

        // Can still access root data
        let result = ctx.resolve(&["page_name".to_string()]).unwrap();
        assert_eq!(result, &json!("Test Page"));

        // And scope data
        let result = ctx
            .resolve(&["person".to_string(), "name".to_string()])
            .unwrap();
        assert_eq!(result, &json!("Jane"));
    }

    #[test]
    fn error_on_missing_key() {
        let data = json!({"name": "Jane"});
        let ctx = RenderContext::new(&data);

        let result = ctx.resolve(&["missing".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn error_on_empty_path() {
        let data = json!({"name": "Jane"});
        let ctx = RenderContext::new(&data);

        let result = ctx.resolve(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty"));
    }
}
