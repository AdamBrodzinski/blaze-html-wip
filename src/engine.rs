use serde_json::Value;
use std::path::PathBuf;

use crate::parse;

#[derive(Debug, Clone)]
pub struct BlazeTemplate {
    pub dev: bool,
    pub(crate) project_path: String,
    pub(crate) root_dir: String,
}

impl Default for BlazeTemplate {
    fn default() -> Self {
        Self {
            dev: false,
            project_path: env!("CARGO_MANIFEST_DIR").to_string(),
            root_dir: "src".to_string(),
        }
    }
}

impl BlazeTemplate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root_directory(mut self, path: &str) -> Self {
        self.root_dir = path.to_string();
        self
    }

    /// Dev mode will disable template caching between requests
    pub fn enable_dev(mut self, dev_enabled: bool) -> Self {
        self.dev = dev_enabled;
        self
    }

    /// transform an HTML page template path into an HTML String
    pub fn render_page(&self, rel_page_path: &str, data: &Value) -> Result<String, String> {
        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parse::parse_template_to_ast(&page_template, data)?;
        let html = parse::render_ast(&ast_nodes, data, page_template.len())?;
        Ok(html)
    }

    pub(crate) fn read_template(&self, rel_page_path: &str) -> Result<String, String> {
        let path_segments = [&self.project_path, &self.root_dir, rel_page_path];
        let template_path = path_segments.iter().collect::<PathBuf>();
        std::fs::read_to_string(&template_path).map_err(|e| e.to_string())
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
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::new()
            .set_root_directory("customer/pages")
            .enable_dev(true);

        assert_eq!(blaze.root_dir, "customer/pages");
        assert_eq!(blaze.dev, true);
    }

    #[test]
    fn fetches_template_and_passes_to_build() {
        let blaze = BlazeTemplate::new().set_root_directory("test_files");
        let data = json!(());
        let result = blaze
            .render_page("pages/test_engine_read.html", &data)
            .unwrap();

        assert_eq!(
            result,
            r#"<script src="pages/test_engine_read.js"></script>
<Style path="pages/test_engine_read.css"/>
<div>Hello World</div>
"#
        );
    }
}
