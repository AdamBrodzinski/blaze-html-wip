use serde_json::Value;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{ast::TemplateNode, error::BlazeError, parser, render};

/// Builder for configuring a [`BlazeTemplate`] instance.
///
/// # Example
/// ```ignore
/// let blaze = BlazeTemplate::builder()
///     .template_root_dir("templates")
///     .dev(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct BlazeTemplateBuilder {
    dev: bool,
    cache_ast: bool,
    template_root_dir: Option<PathBuf>,
}

impl Default for BlazeTemplateBuilder {
    fn default() -> Self {
        Self {
            dev: false,
            cache_ast: true,
            template_root_dir: None,
        }
    }
}

impl BlazeTemplateBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root directory for templates.
    ///
    /// Paths are stored as-is and resolved relative to cwd at runtime.
    /// Default is `"."` (current working directory).
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

    /// Enable or disable AST caching.
    /// Default is `true`. When disabled, templates are parsed on every render.
    pub fn cache_ast(mut self, enabled: bool) -> Self {
        self.cache_ast = enabled;
        self
    }

    /// Build the `BlazeTemplate` instance.
    pub fn build(self) -> BlazeTemplate {
        let template_root_dir = self.template_root_dir.unwrap_or_else(|| PathBuf::from("."));

        BlazeTemplate {
            inner: Arc::new(BlazeTemplateInner {
                dev: self.dev,
                cache_ast: self.cache_ast,
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
    cache_ast: bool,
    template_root_dir: PathBuf,
    ast_nodes: RwLock<HashMap<String, (Vec<TemplateNode>, usize)>>,
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
    pub fn compile_page_template(&self, rel_page_path: &str) -> crate::error::Result<()> {
        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parser::parse_template_to_ast(&page_template)?;
        if !self.inner.dev && self.inner.cache_ast {
            self.set_cached_ast(rel_page_path, ast_nodes, page_template.len());
        }
        Ok(())
    }

    /// Render a template file to an HTML string.
    pub fn render_page(&self, rel_page_path: &str, data: &Value) -> crate::error::Result<String> {
        let should_cache = !self.inner.dev && self.inner.cache_ast;

        if should_cache {
            match self.inner.ast_nodes.read() {
                Ok(cache) => {
                    if let Some((cached_ast, template_len)) = cache.get(rel_page_path) {
                        return render::render_ast(cached_ast, data, *template_len);
                    }
                }
                Err(_) => {
                    // Lock poisoned, clear the potentially inconsistent cache
                    if self.is_dev() {
                        println!("Lock poisoned, clearing cache");
                    }
                    self.clear_cache();
                }
            }
        }

        let page_template = self.read_template(rel_page_path)?;
        let ast_nodes = parser::parse_template_to_ast(&page_template)?;
        let template_len = page_template.len();

        let result = render::render_ast(&ast_nodes, data, template_len);

        if should_cache {
            self.set_cached_ast(rel_page_path, ast_nodes, template_len);
        }

        result
    }

    fn read_template(&self, rel_page_path: &str) -> crate::error::Result<String> {
        let template_path = self.inner.template_root_dir.join(rel_page_path);
        std::fs::read_to_string(&template_path)
            .map_err(|e| BlazeError::template_io(&template_path, e))
    }

    fn set_cached_ast(&self, rel_page_path: &str, ast: Vec<TemplateNode>, len: usize) {
        let mut cache = self.write_cache_or_clear();
        cache.insert(rel_page_path.to_string(), (ast, len));
    }

    fn clear_cache(&self) {
        self.write_cache_or_clear().clear();
    }

    fn write_cache_or_clear(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, HashMap<String, (Vec<TemplateNode>, usize)>> {
        match self.inner.ast_nodes.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                guard
            }
        }
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
        assert_eq!(blaze.template_root_dir(), std::path::Path::new("."));
        assert_eq!(blaze.is_dev(), false);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("customer/pages")
            .dev(true)
            .build();

        assert_eq!(
            blaze.template_root_dir(),
            std::path::Path::new("customer/pages")
        );
        assert_eq!(blaze.is_dev(), true);
    }

    #[test]
    fn clone_shares_cache() {
        let blaze1 = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
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
            let blaze = BlazeTemplate::builder()
                .template_root_dir("test_files")
                .build();
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
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        blaze
            .compile_page_template("pages/test_engine_read.html")
            .unwrap();

        let ast_node_len = blaze.inner.ast_nodes.read().unwrap().len();
        assert_eq!(ast_node_len, 1);
    }

    #[test]
    fn render_page_populates_cache() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        let data = json!(());

        // First render should populate cache
        blaze
            .render_page("pages/test_engine_read.html", &data)
            .unwrap();
        assert_eq!(blaze.inner.ast_nodes.read().unwrap().len(), 1);
    }

    #[test]
    fn cache_ast_disabled_skips_caching() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .cache_ast(false)
            .build();
        let data = json!(());

        blaze
            .render_page("pages/test_engine_read.html", &data)
            .unwrap();
        assert_eq!(blaze.inner.ast_nodes.read().unwrap().len(), 0);
    }

    #[test]
    fn dev_mode_skips_caching() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .dev(true)
            .build();
        let data = json!(());

        blaze
            .render_page("pages/test_engine_read.html", &data)
            .unwrap();
        assert_eq!(blaze.inner.ast_nodes.read().unwrap().len(), 0);
    }
}
