use serde_json::Value;

/// A value bound into a scope layer.
///
/// `Borrowed` points into the `'a` data (root data or an `Each` element); it
/// carries the data lifetime, so it can be handed back out of [`RenderContext`]
/// without cloning. `Owned` holds a value the context owns (a static component
/// prop) and can only be lent for as long as the context lives.
pub(crate) enum ScopeValue<'a> {
    Borrowed(&'a Value),
    Owned(Value),
}

impl<'a> ScopeValue<'a> {
    fn as_value(&self) -> &Value {
        match self {
            ScopeValue::Borrowed(value) => value,
            ScopeValue::Owned(value) => value,
        }
    }

    /// The underlying `&'a Value` when this layer borrows from the data, else
    /// `None` for an owned layer.
    fn borrowed(&self) -> Option<&'a Value> {
        match self {
            ScopeValue::Borrowed(value) => Some(value),
            ScopeValue::Owned(_) => None,
        }
    }
}

/// Result of [`RenderContext::resolve_borrowed`].
pub(crate) enum Resolved<'a> {
    /// Value backed by the `'a` data — usable without cloning.
    Ref(&'a Value),
    /// Match landed in an owned scope (a static prop); the caller may clone via
    /// [`RenderContext::resolve`] if it needs ownership.
    Owned,
}

/// each layer in the variable scope chain
struct ScopeLayer<'a> {
    name: String,          // <Each items="foo" name
    value: ScopeValue<'a>, // borrowed data or an owned prop value
}

/// Render context with scope chain for variable resolution.
/// The scope chain is searched from innermost (last) to outermost (first).
/// The root_data is the fallback for top-level variables.
///
/// Scope values are stored *by reference* into the `'a` data wherever possible
/// (`Each` bindings, variable props), so rendering never deep-clones the input
/// JSON. The only owned entries are static component props (small, literal
/// strings from the template).
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

    pub fn root_data(&self) -> &'a Value {
        self.root_data
    }

    pub fn push_scope(&mut self, name: &str, value: ScopeValue<'a>) {
        self.scope_stack.push(ScopeLayer {
            name: name.to_string(),
            value,
        });
    }

    /// Push a scope that borrows from the `'a` data (no clone).
    pub fn push_scope_ref(&mut self, name: &str, value: &'a Value) {
        self.push_scope(name, ScopeValue::Borrowed(value));
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
    pub fn resolve(&self, segments: &[String]) -> Result<&Value, String> {
        if segments.is_empty() {
            return Err("Empty variable path".to_string());
        }

        let first_segment = &segments[0];

        // search scope stack from innermost to outermost
        for scope in self.scope_stack.iter().rev() {
            // "person" in ["person", "name"] or in ["person"]
            if scope.name == *first_segment {
                // segments is ["person"], exact match for "person"
                if segments.len() == 1 {
                    return Ok(scope.value.as_value());
                }
                // segments is ["person", "name"], matched "person", pass remaining ["name"] to resolve remaining path
                return resolve_path(scope.value.as_value(), &segments[1..]);
            }
        }

        // not in any scope - resolve from root data
        resolve_path(self.root_data, segments)
    }

    /// Like [`resolve`](Self::resolve), but yields a reference carrying the data
    /// lifetime `'a` when the value is backed by borrowed data (the root data or
    /// an `Each` binding). When the match lands in an owned scope (a static prop)
    /// it returns [`Resolved::Owned`] so the caller can clone only if it must.
    ///
    /// This is what lets `Each` hold onto the array while pushing per-item scopes,
    /// and lets variable props forward by reference — both without cloning the
    /// underlying JSON.
    pub fn resolve_borrowed(&self, segments: &[String]) -> Result<Resolved<'a>, String> {
        if segments.is_empty() {
            return Err("Empty variable path".to_string());
        }

        let first_segment = &segments[0];

        for scope in self.scope_stack.iter().rev() {
            if scope.name == *first_segment {
                let Some(value) = scope.value.borrowed() else {
                    // matched an owned (static prop) layer
                    return Ok(Resolved::Owned);
                };
                // `value` is `&'a Value`, so the result keeps the `'a` lifetime
                // rather than borrowing `self`.
                if segments.len() == 1 {
                    return Ok(Resolved::Ref(value));
                }
                return resolve_path(value, &segments[1..]).map(Resolved::Ref);
            }
        }

        resolve_path(self.root_data, segments).map(Resolved::Ref)
    }
}

/// access nested data {"person": {"name": "Jane"}} with ["person", "name"] == Ok("Jane")
fn resolve_path<'a>(data: &'a Value, segments: &[String]) -> Result<&'a Value, String> {
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
        ctx.push_scope_ref("person", person);

        // Should resolve from scope
        let result = ctx.resolve(&["person".to_string(), "name".to_string()]);
        assert_eq!(result.unwrap(), &json!("Jane"));
    }

    #[test]
    fn scope_shadows_root() {
        let data = json!({"name": "Root", "person": {"name": "Nested"}});
        let nested = &data["person"];

        let mut ctx = RenderContext::new(&data);
        ctx.push_scope_ref("name", nested);

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
        ctx.push_scope_ref("item", outer);
        ctx.push_scope_ref("item", inner);

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
        ctx.push_scope_ref("person", person);

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
