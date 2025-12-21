use serde_json::Value;

pub struct BlazeTemplate {
    components_dir: String,
    layouts_dir: String,
    pages_dir: String,
    panic_on_error: bool,
}

impl BlazeTemplate {
    pub fn new() -> Self {
        Self {
            components_dir: "src/components".to_string(),
            layouts_dir: "src/layouts".to_string(),
            pages_dir: "src/pages".to_string(),
            panic_on_error: false,
        }
    }

    pub fn register_components_directory(mut self, dir: &str) -> Self {
        self.components_dir = dir.to_string();
        self
    }

    pub fn register_layouts_directory(mut self, dir: &str) -> Self {
        self.layouts_dir = dir.to_string();
        self
    }

    pub fn register_pages_directory(mut self, dir: &str) -> Self {
        self.pages_dir = dir.to_string();
        self
    }

    pub fn panic_on_error(mut self, should_panic: bool) -> Self {
        self.panic_on_error = should_panic;
        self
    }

    pub fn render_page(&self, _data: Value, _page: &str) -> String {
        "TODO".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_with_defaults() {
        let blaze = BlazeTemplate::new();
        assert_eq!(blaze.components_dir, "src/components");
        assert_eq!(blaze.layouts_dir, "src/layouts");
        assert_eq!(blaze.pages_dir, "src/pages");
        assert_eq!(blaze.panic_on_error, false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::new()
            .register_components_directory("custom/components")
            .register_layouts_directory("custom/layouts")
            .register_pages_directory("custom/pages")
            .panic_on_error(true);

        assert_eq!(blaze.components_dir, "custom/components");
        assert_eq!(blaze.layouts_dir, "custom/layouts");
        assert_eq!(blaze.pages_dir, "custom/pages");
        assert_eq!(blaze.panic_on_error, true);
    }

    #[test]
    fn test_render_page_returns_todo() {
        let blaze = BlazeTemplate::new();
        let data = json!({"title": "Home"});
        let result = blaze.render_page(data, "customers/view.html");
        assert_eq!(result, "TODO");
    }
}
