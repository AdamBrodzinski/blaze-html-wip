//! Variable scoping for template rendering.
//!
//! [`RenderContext`] resolves a dotted variable path like `@person.name` to a value
//! in the data being rendered. It holds a *scope chain*: a stack of named bindings
//! layered on top of the root JSON passed to the render function.
//!
//! Two constructs introduce a binding:
//!
//! - `<Each items="@people" as="person">` pushes `person` for each item it renders.
//! - A component prop — `<Card title="...">` — pushes `title` while that component renders.
//!
//! Resolution walks the chain from the innermost (most recently pushed) binding
//! outward, so an inner binding shadows an outer one of the same name. If no binding
//! matches the first path segment, the path is resolved against the root data instead.
//!
//! ```text
//! data:     { "site": "Blog", "posts": [ { "title": "Hi" } ] }
//! template: <Each items="@posts" as="post">@post.title on @site</Each>
//!
//! while rendering an item the chain is:  [ post -> {"title": "Hi"} ]  over root { site, posts }
//!   @post.title  ->  matches `post`, then ."title"        =>  "Hi"
//!   @site        ->  no binding named `site`, hits root   =>  "Blog"
//! ```
//!
//! Every name and value in the chain is borrowed (`&'a str` / `&'a Value`) from either
//! the input JSON or the parsed (and cached) template AST, so resolving and iterating
//! never clone the data or allocate a scope name.

use serde_json::Value;

/// A scope layer is created when entering a nested lexical scope in the template. The name
/// comes from the <Each> 'as' attribute name OR any component attribute. These are the only two
/// ways the stack changes. push_scope adds a binding when entering a nested component or each
/// and then pop_scope removes it when leaving that component/each block.
struct ScopeLayer<'a> {
    name: &'a str,    // binding name, e.g. <Each items="foo" as="item"> or a component prop
    value: &'a Value, // borrowed from the input JSON OR the parsed AST
}

/// Render context with scope chain for variable resolution.
/// The scope chain is searched from innermost (last) to outermost (first).
/// The root_data is the fallback for top-level variables.
///
/// Both scope names and scope values are borrows (`&'a str` / `&'a Value`) into the
/// `'a` data or the parsed AST, so rendering never deep-clones the input JSON nor
/// allocates a scope name on each push.
/// ----
///

// Picture it like this. root_data is the floor; the stack grows upward as you enter nested scopes:
//
//             ┌──────────────────────────────┐
//  innermost  │ [2] name="skill" value=◆ ────┼──► "Rust"      ← searched FIRST
//             ├──────────────────────────────┤
//             │ [1] name="post"  value=◆ ────┼──► {"title":..}
//             ├──────────────────────────────┤
//  outermost  │ [0] name="user"  value=◆ ────┼──► {"name":..}
//             └──────────────────────────────┘
//                                                   ↑ if nothing matches, fall through to:
//             root_data ──────────────────────────► { the whole JSON object }
//
// The order matters: the most recently pushed layer is at the top, and resolution searches from the top down. That single rule gives us shadowing (inner names
// beating outer names) for free — we'll see how.

pub struct RenderContext<'a> {
    root_data: &'a Value,             // json data passed into render fn
    scope_stack: Vec<ScopeLayer<'a>>, // each lexical scope,
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

    /// Push a scope binding. Both the name and value borrow from the `'a` data or
    /// the parsed AST — no allocation.
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
    ///
    /// The result carries the data lifetime `'a` (not tied to `&self`), so callers
    /// can hold the returned reference while continuing to push/pop scopes — this is
    /// what lets `<Each>` keep the array borrow while iterating, and lets variable
    /// props forward by reference, both without cloning.
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

        // not in any scope - resolve from root data
        resolve_path(self.root_data, segments)
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
