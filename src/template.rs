use serde_json::Value;

pub struct BlazeTemplate {
    absolute_project_path: String,
    components_dir: String,
    dev: bool,
    layouts_dir: String,
    pages_dir: String,
    panic_on_error: bool,
}

impl BlazeTemplate {
    pub fn new() -> Self {
        Self {
            absolute_project_path: env!("CARGO_MANIFEST_DIR").to_string(),
            dev: false,
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

    pub fn enable_dev(mut self, dev_enabled: bool) -> Self {
        self.dev = dev_enabled;
        self
    }

    pub fn render_page(&self, _data: Value, rel_page_path: &str) -> String {
        let template_path = format!(
            "{}/{}/{}",
            self.absolute_project_path, self.pages_dir, rel_page_path
        );
        dbg!(&template_path);

        let template_file = std::fs::read_to_string(template_path).unwrap();
        template_file
    }
}

impl Default for BlazeTemplate {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::bool_assert_comparison)]
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
        assert_eq!(blaze.dev, false);
        assert_eq!(blaze.panic_on_error, false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::new()
            .register_components_directory("custom/components")
            .register_layouts_directory("custom/layouts")
            .register_pages_directory("custom/pages")
            .enable_dev(true)
            .panic_on_error(true);

        assert_eq!(blaze.components_dir, "custom/components");
        assert_eq!(blaze.layouts_dir, "custom/layouts");
        assert_eq!(blaze.pages_dir, "custom/pages");
        assert_eq!(blaze.dev, true);
        assert_eq!(blaze.panic_on_error, true);
    }

    #[test]
    fn renders_static_html_without_template() {
        let blaze = BlazeTemplate::new().register_pages_directory("test_files");
        let data = json!(());
        let result = blaze.render_page(data, "pages/static.html");
        assert_eq!(result, "<div>Hello World</div>\n");
    }
}
