use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{ast::TemplateNode, parse};

/// HTML template engine
///
/// # Example
/// ```ignore
/// let blaze = BlazeTemplate::new()
///     .set_root_directory("templates")
///     .enable_dev(true);
///
/// blaze.render_page("pages/about.html", json_data);
/// ```
#[derive(Clone)]
pub struct BlazeTemplate {
    inner: Arc<BlazeTemplateInner>,
}

struct BlazeTemplateInner {
    dev: bool,
    project_path: String,
    root_dir: String,
    ast_nodes: RwLock<HashMap<String, Vec<TemplateNode>>>,
}

impl Clone for BlazeTemplateInner {
    fn clone(&self) -> Self {
        Self {
            dev: self.dev,
            project_path: self.project_path.clone(),
            root_dir: self.root_dir.clone(),
            ast_nodes: RwLock::new(HashMap::new()),
        }
    }
}

impl std::fmt::Debug for BlazeTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlazeTemplate")
            .field("dev", &self.inner.dev)
            .field("project_path", &self.inner.project_path)
            .field("root_dir", &self.inner.root_dir)
            .finish_non_exhaustive()
    }
}

impl Default for BlazeTemplate {
    fn default() -> Self {
        let cwd = std::env::current_dir().expect("Expected the current directory to be found");
        let project_path = cwd.to_string_lossy().into_owned();
        Self {
            inner: Arc::new(BlazeTemplateInner {
                dev: false,
                project_path,
                root_dir: "src".to_string(),
                ast_nodes: RwLock::new(HashMap::new()),
            }),
        }
    }
}

impl BlazeTemplate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root directory for templates, relative to the project path.
    pub fn set_root_directory(mut self, path: &str) -> Self {
        Arc::make_mut(&mut self.inner).root_dir = path.to_string();
        self
    }

    /// Dev mode will disable template caching between requests.
    pub fn enable_dev(mut self, dev_enabled: bool) -> Self {
        Arc::make_mut(&mut self.inner).dev = dev_enabled;
        self
    }

    /// Returns whether dev mode is enabled.
    pub fn is_dev(&self) -> bool {
        self.inner.dev
    }

    /// Returns the configured root directory.
    pub fn root_dir(&self) -> &str {
        &self.inner.root_dir
    }

    /// Pre-compile a template and cache its AST for faster rendering.
    ///
    /// This is useful for warming up the cache at application startup.
    pub fn compile_page_template(&self, rel_page_path: &str) -> Result<(), String> {
        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parse::parse_template_to_ast(&page_template, &json!(()))?;
        if !self.inner.dev {
            self.inner
                .ast_nodes
                .write()
                .map_err(|e| format!("AST cache write lock poisoned: {e}"))?
                .insert(rel_page_path.to_string(), ast_nodes);
        }
        Ok(())
    }

    /// Render a template file to an HTML string.
    pub fn render_page(&self, rel_page_path: &str, data: &Value) -> Result<String, String> {
        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parse::parse_template_to_ast(&page_template, data)?;
        let html = parse::render_ast(&ast_nodes, data, page_template.len())?;
        Ok(html)
    }

    fn read_template(&self, rel_page_path: &str) -> Result<String, String> {
        let path_segments = [
            self.inner.project_path.as_str(),
            self.inner.root_dir.as_str(),
            rel_page_path,
        ];
        let template_path = path_segments.iter().collect::<PathBuf>();
        std::fs::read_to_string(&template_path).map_err(|e| e.to_string())
    }
}

#[allow(clippy::bool_assert_comparison)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_defaults() {
        let blaze = BlazeTemplate::new();
        assert_eq!(blaze.root_dir(), "src");
        assert_eq!(blaze.is_dev(), false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::new()
            .set_root_directory("customer/pages")
            .enable_dev(true);

        assert_eq!(blaze.root_dir(), "customer/pages");
        assert_eq!(blaze.is_dev(), true);
    }

    #[test]
    fn clone_shares_cache() {
        let blaze1 = BlazeTemplate::new().set_root_directory("test_files");
        let blaze2 = blaze1.clone();

        // Both point to the same inner Arc
        assert!(Arc::ptr_eq(&blaze1.inner, &blaze2.inner));

        // Compile on one, visible on the other
        blaze1
            .compile_page_template("pages/test_engine_read.html")
            .unwrap();

        let cache_len = blaze2.inner.ast_nodes.read().unwrap().len();
        assert_eq!(cache_len, 1);
    }

    #[cfg(feature = "cache-bust")]
    mod render_with_hash {
        use super::*;
        use indoc::formatdoc;
        use serde_json::json;

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
    }

    #[test]
    fn compile_page_caches_ast() {
        let blaze = BlazeTemplate::new().set_root_directory("test_files");
        blaze
            .compile_page_template("pages/test_engine_read.html")
            .unwrap();

        let ast_node_len = blaze.inner.ast_nodes.read().unwrap().len();
        assert_eq!(ast_node_len, 1);
    }
}
