use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{ast::TemplateNode, parse};

#[derive(Debug, Clone)]
pub struct BlazeTemplate {
    pub dev: bool,
    pub(crate) project_path: String,
    pub(crate) root_dir: String,
    pub(crate) ast_nodes: Arc<RwLock<HashMap<String, Vec<TemplateNode>>>>,
}

impl Default for BlazeTemplate {
    fn default() -> Self {
        let cwd = std::env::current_dir().expect("Expected the current directory to be found");
        let project_path = cwd.to_string_lossy().into_owned();
        Self {
            dev: false,
            project_path,
            root_dir: "src".to_string(),
            ast_nodes: Arc::new(RwLock::new(HashMap::new())),
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
    pub fn compile_page_template(&self, rel_page_path: &str) -> Result<(), String> {
        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parse::parse_template_to_ast(&page_template, &json!(()))?;
        if !self.dev {
            self.ast_nodes
                .write()
                .map_err(|e| format!("AST cache write lock poisoned: {e}"))?
                .insert(rel_page_path.to_string(), ast_nodes);
        }
        Ok(())
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
    use indoc::formatdoc;
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

    const JS_HASH: &str = "44f4b32954b6da985de1cfd924eacdda";
    const CSS_HASH: &str = "5c7ead8de806c5ed42f44b22c63183ee";

    #[test]
    fn fetches_template_and_passes_to_build() {
        let blaze = BlazeTemplate::new().set_root_directory("test_files");
        let data = json!(());
        let result = blaze
            .render_page("pages/test_engine_read.html", &data)
            .unwrap();

        let expected = formatdoc! {r#"
            <script src="test_files/pages/test_engine_read.js?{JS_HASH}"></script>
            <link rel="stylesheet" href="test_files/pages/test_engine_read.css?{CSS_HASH}">
            <div>Hello World</div>
        "#};
        assert_eq!(result, expected);
    }

    #[test]
    fn compile_page_caches_ast() {
        let blaze = BlazeTemplate::new().set_root_directory("test_files");
        blaze
            .compile_page_template("pages/test_engine_read.html")
            .unwrap();

        let ast_node_len = blaze.ast_nodes.read().unwrap().len();
        assert_eq!(ast_node_len, 1);
    }
}
