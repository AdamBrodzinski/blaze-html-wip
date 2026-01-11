use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{ast::TemplateNode, parse};

/// Builder for configuring a [`BlazeTemplate`] instance.
///
/// # Example
/// ```ignore
/// let blaze = BlazeTemplate::builder()
///     .template_root_dir("templates")
///     .dev(true)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct BlazeTemplateBuilder {
    dev: bool,
    template_root_dir: Option<PathBuf>,
}

impl BlazeTemplateBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root directory for templates.
    ///
    /// Relative paths are resolved against the current working directory at build time.
    /// Default is the current working directory.
    pub fn template_root_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.template_root_dir = Some(path.into());
        self
    }

    /// Enable dev mode, which disables template caching between requests.
    /// Default is `false`.
    pub fn dev(mut self, enabled: bool) -> Self {
        self.dev = enabled;
        self
    }

    /// Build the `BlazeTemplate` instance.
    ///
    /// # Panics
    /// Panics if the current working directory cannot be determined.
    pub fn build(self) -> BlazeTemplate {
        let cwd = std::env::current_dir().expect("could not determine current directory");

        let template_root_dir = match self.template_root_dir {
            Some(path) if path.is_absolute() => path,
            Some(path) => cwd.join(path),
            None => cwd,
        };

        BlazeTemplate {
            inner: Arc::new(BlazeTemplateInner {
                dev: self.dev,
                template_root_dir,
                ast_nodes: RwLock::new(HashMap::new()),
            }),
        }
    }
}

/// HTML template engine with caching support.
///
/// # Example
/// ```ignore
/// let blaze = BlazeTemplate::builder()
///     .template_root_dir("templates")
///     .dev(true)
///     .build();
///
/// blaze.render_page("pages/about.html", &json!({}));
/// ```
#[derive(Clone)]
pub struct BlazeTemplate {
    inner: Arc<BlazeTemplateInner>,
}

struct BlazeTemplateInner {
    dev: bool,
    template_root_dir: PathBuf,
    ast_nodes: RwLock<HashMap<String, Vec<TemplateNode>>>,
}

impl std::fmt::Debug for BlazeTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlazeTemplate")
            .field("dev", &self.inner.dev)
            .field("template_root_dir", &self.inner.template_root_dir)
            .finish_non_exhaustive()
    }
}

impl Default for BlazeTemplate {
    fn default() -> Self {
        BlazeTemplateBuilder::new().build()
    }
}

impl BlazeTemplate {
    /// Create a new `BlazeTemplate` with default settings.
    ///
    /// For custom configuration, use [`BlazeTemplate::builder()`] instead.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for configuring a `BlazeTemplate`.
    pub fn builder() -> BlazeTemplateBuilder {
        BlazeTemplateBuilder::new()
    }

    /// Returns whether dev mode is enabled.
    pub fn is_dev(&self) -> bool {
        self.inner.dev
    }

    /// Returns the configured template root directory.
    pub fn template_root_dir(&self) -> &std::path::Path {
        &self.inner.template_root_dir
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
        let template_path = self.inner.template_root_dir.join(rel_page_path);
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
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(blaze.template_root_dir(), cwd);
        assert_eq!(blaze.is_dev(), false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("customer/pages")
            .dev(true)
            .build();

        assert!(blaze.template_root_dir().ends_with("customer/pages"));
        assert!(blaze.template_root_dir().is_absolute());
        assert_eq!(blaze.is_dev(), true);
    }

    #[test]
    fn clone_shares_cache() {
        let blaze1 = BlazeTemplate::builder().template_root_dir("test_files").build();
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
            let blaze = BlazeTemplate::builder().template_root_dir("test_files").build();
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
        let blaze = BlazeTemplate::builder().template_root_dir("test_files").build();
        blaze
            .compile_page_template("pages/test_engine_read.html")
            .unwrap();

        let ast_node_len = blaze.inner.ast_nodes.read().unwrap().len();
        assert_eq!(ast_node_len, 1);
    }
}
