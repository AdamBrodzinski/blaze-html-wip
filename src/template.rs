use std::path::PathBuf;

use serde_json::Value;

use crate::build_template::build_template;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlazeTemplate {
    dev: bool,
    panic_on_error: bool,
    project_path: String,
    root_dir: String,
}

impl BlazeTemplate {
    pub fn new() -> Self {
        Self {
            dev: false,
            panic_on_error: false,
            project_path: env!("CARGO_MANIFEST_DIR").to_string(),
            root_dir: "src".to_string(),
        }
    }

    pub fn set_root_directory(mut self, path: &str) -> Self {
        self.root_dir = path.to_string();
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

    pub fn render_page(&self, _data: Value, rel_page_path: &str) -> Result<String, String> {
        let template_path: PathBuf = [&self.project_path, &self.root_dir, rel_page_path]
            .iter()
            .collect();
        let template_file = std::fs::read_to_string(template_path).map_err(|e| e.to_string())?;
        build_template(self, &template_file)
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
        assert_eq!(blaze.root_dir, "src");
        assert_eq!(blaze.dev, false);
        assert_eq!(blaze.panic_on_error, false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::new()
            .set_root_directory("customer/pages")
            .enable_dev(true)
            .panic_on_error(true);

        assert_eq!(blaze.root_dir, "customer/pages");
        assert_eq!(blaze.dev, true);
        assert_eq!(blaze.panic_on_error, true);
    }

    #[test]
    fn fetches_template_and_passes_to_build() {
        let blaze = BlazeTemplate::new().set_root_directory("test_files");
        let data = json!(());
        let result = blaze.render_page(data, "pages/static.html").unwrap();
        assert_eq!(result, "<div>Hello World</div>\n");
    }
}
