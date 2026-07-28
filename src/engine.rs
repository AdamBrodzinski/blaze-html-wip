use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    ast::TemplateNode,
    error::BlazeError,
    parser,
    render::{self, ComponentResolver, ComponentTemplate},
};

/// Builder for configuring a [`BlazeTemplate`] instance.
///
/// # Example
///
/// ```no_run
/// use blaze_html::{BlazeTemplate, Result};
///
/// # fn main() -> Result<()> {
/// let blaze = BlazeTemplate::builder()
///     .template_root_dir("templates")
///     .register_components([
///         ("AppLayout", "layouts/app_layout.html"),
///         ("EmptyLayout", "layouts/empty_layout.html"),
///     ])?
///     .dev(true)
///     .build();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct BlazeTemplateBuilder {
    dev: bool,
    template_root_dir: Option<PathBuf>,
    components: HashMap<String, PathBuf>,
}

impl BlazeTemplateBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root directory for page templates, components, and includes.
    ///
    /// A relative root is resolved from the process working directory. Asset tag
    /// paths are not affected by this setting. The default is `"."`.
    pub fn template_root_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.template_root_dir = Some(path.into());
        self
    }

    /// Register the components available to templates.
    ///
    /// Component paths are resolved relative to [`Self::template_root_dir`].
    /// If a name occurs more than once, the last path replaces the earlier one.
    ///
    /// # Errors
    ///
    /// Returns [`BlazeError::Render`] if a component name is invalid or reserved.
    pub fn register_components<I, N, P>(mut self, components: I) -> crate::error::Result<Self>
    where
        I: IntoIterator<Item = (N, P)>,
        N: Into<String>,
        P: Into<PathBuf>,
    {
        for (name, path) in components {
            let name = name.into();
            validate_component_name(&name)?;
            self.components.insert(name, path.into());
        }
        Ok(self)
    }

    /// Enable development mode.
    ///
    /// Development mode disables page AST, component AST, and include-content
    /// caching so file changes appear on the next render. The default is `false`.
    pub fn dev(mut self, enabled: bool) -> Self {
        self.dev = enabled;
        self
    }

    /// Build the `BlazeTemplate` instance.
    pub fn build(self) -> BlazeTemplate {
        let template_root_dir = self.template_root_dir.unwrap_or_else(|| PathBuf::from("."));

        BlazeTemplate {
            inner: Arc::new(BlazeTemplateInner {
                dev: self.dev,
                template_root_dir,
                ast_nodes: RwLock::new(HashMap::new()),
                components: self.components,
                component_ast: RwLock::new(HashMap::new()),
                include_cache: RwLock::new(HashMap::new()),
            }),
        }
    }
}

/// HTML template engine with caching support.
///
/// Cloning an engine is inexpensive: clones share the same page, component, and
/// include caches. Page templates, components, and includes are resolved relative
/// to the configured template root. Asset tag paths are instead resolved from the
/// process working directory and are not affected by the template root.
///
/// # Example
///
/// ```no_run
/// use blaze_html::{BlazeTemplate, Result};
/// use serde_json::json;
///
/// # fn main() -> Result<()> {
/// let blaze = BlazeTemplate::builder()
///     .template_root_dir("templates")
///     .dev(true)
///     .build();
///
/// let home = blaze.render_page("pages/home.html", &json!({ "name": "Ada" }))?;
/// let about = blaze.render_page_static("pages/about.html")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct BlazeTemplate {
    inner: Arc<BlazeTemplateInner>,
}

struct BlazeTemplateInner {
    dev: bool,
    template_root_dir: PathBuf,
    ast_nodes: RwLock<HashMap<PathBuf, (Vec<TemplateNode>, usize)>>,
    components: HashMap<String, PathBuf>,
    component_ast: RwLock<HashMap<String, Arc<ComponentTemplate>>>,
    include_cache: RwLock<HashMap<String, Arc<String>>>,
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

    /// Returns the root directory used for page templates, components, and includes.
    ///
    /// Asset tag paths are not resolved relative to this directory.
    pub fn template_root_dir(&self) -> &std::path::Path {
        &self.inner.template_root_dir
    }

    /// Validate page templates and cache their ASTs for faster rendering.
    ///
    /// Templates are processed in iteration order, and processing stops at the first
    /// error. Outside development mode, each successful template is cached immediately,
    /// making this useful for warming the cache at application startup. In development
    /// mode, templates are still read, parsed, and validated but are not cached.
    ///
    /// # Errors
    ///
    /// Returns an error if a page cannot be read or parsed, or if it references an
    /// unregistered component.
    pub fn compile_page_templates<I, P>(&self, rel_page_paths: I) -> crate::error::Result<()>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let should_cache = !self.inner.dev;

        for rel_page_path in rel_page_paths {
            let rel_page_path = rel_page_path.as_ref();
            let page_template = self.read_template(rel_page_path)?;
            let ast_nodes = parser::parse_template_to_ast(&page_template)?;
            self.validate_component_references(&ast_nodes)?;

            if should_cache {
                self.set_cached_ast(rel_page_path, ast_nodes, page_template.len());
            }
        }

        Ok(())
    }

    /// Render a template file to an HTML string.
    ///
    /// Accepts any [`Serialize`] type, so view models can be passed directly
    /// without converting to [`serde_json::Value`] first.
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be serialized; a page, component, or
    /// include cannot be read or parsed; template data cannot be resolved or rendered;
    /// or an asset cannot be hashed.
    pub fn render_page(
        &self,
        rel_page_path: &str,
        data: &impl Serialize,
    ) -> crate::error::Result<String> {
        let data = serde_json::to_value(data)?;
        let should_cache = !self.inner.dev;

        if should_cache {
            match self.inner.ast_nodes.read() {
                Ok(cache) => {
                    if let Some((cached_ast, template_len)) = cache.get(Path::new(rel_page_path)) {
                        return render::render_ast(cached_ast, &data, *template_len, self);
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

        let result = render::render_ast(&ast_nodes, &data, template_len, self);

        if should_cache {
            self.set_cached_ast(Path::new(rel_page_path), ast_nodes, template_len);
        }

        result
    }

    /// Render a template file that takes no data.
    ///
    /// Equivalent to [`Self::render_page`] with an empty data set. Templates that
    /// reference a variable still error, since there is nothing to resolve against.
    ///
    /// # Errors
    ///
    /// Returns an error if the page or one of its dependencies cannot be read,
    /// parsed, or rendered. This includes templates that reference variables.
    pub fn render_page_static(&self, rel_page_path: &str) -> crate::error::Result<String> {
        // An empty object rather than `()`/null, so a template that does reference a
        // variable reports "not found in object" instead of "non-object value".
        self.render_page(rel_page_path, &serde_json::Map::new())
    }

    fn read_template(&self, rel_page_path: impl AsRef<Path>) -> crate::error::Result<String> {
        let template_path = self.inner.template_root_dir.join(rel_page_path);
        std::fs::read_to_string(&template_path)
            .map_err(|e| BlazeError::template_io(&template_path, e))
    }

    fn get_component_template(&self, name: &str) -> crate::error::Result<Arc<ComponentTemplate>> {
        let should_cache = !self.inner.dev;

        if should_cache {
            match self.inner.component_ast.read() {
                Ok(cache) => {
                    if let Some(template) = cache.get(name) {
                        return Ok(template.clone());
                    }
                }
                Err(_) => {
                    if self.is_dev() {
                        println!("Lock poisoned, clearing component cache");
                    }
                    self.clear_component_cache();
                }
            }
        }

        let rel_path = self.component_path(name)?;
        let template = self.read_template(rel_path)?;
        let ast_nodes = parser::parse_template_to_ast(&template)?;
        self.validate_component_references(&ast_nodes)?;

        let component = Arc::new(ComponentTemplate {
            ast: ast_nodes,
            template_len: template.len(),
        });

        if should_cache {
            let mut cache = self.write_component_cache_or_clear();
            cache.insert(name.to_string(), component.clone());
        }

        Ok(component)
    }

    /// read the raw text from a file relative to the template root path. contents are cached in
    /// AST (except dev mode).
    fn get_include(&self, rel_path: &str) -> crate::error::Result<Arc<String>> {
        let should_cache = !self.inner.dev;

        if should_cache {
            match self.inner.include_cache.read() {
                Ok(cache) => {
                    if let Some(contents) = cache.get(rel_path) {
                        return Ok(contents.clone());
                    }
                }
                Err(_) => {
                    if self.is_dev() {
                        println!("Lock poisoned, clearing include cache");
                    }
                    self.clear_include_cache();
                }
            }
        }

        let full_path = self.inner.template_root_dir.join(rel_path);
        let contents = Arc::new(
            std::fs::read_to_string(&full_path)
                .map_err(|e| BlazeError::include_io(&full_path, e))?,
        );

        if should_cache {
            let mut cache = self.write_include_cache_or_clear();
            cache.insert(rel_path.to_string(), contents.clone());
        }

        Ok(contents)
    }

    fn set_cached_ast(&self, rel_page_path: &Path, ast: Vec<TemplateNode>, len: usize) {
        let mut cache = self.write_cache_or_clear();
        cache.insert(rel_page_path.to_path_buf(), (ast, len));
    }

    fn clear_cache(&self) {
        self.write_cache_or_clear().clear();
    }

    fn component_path(&self, name: &str) -> crate::error::Result<&Path> {
        self.inner
            .components
            .get(name)
            .map(PathBuf::as_path)
            .ok_or_else(|| BlazeError::render(name, "component not registered"))
    }

    fn clear_component_cache(&self) {
        self.write_component_cache_or_clear().clear();
    }

    fn clear_include_cache(&self) {
        self.write_include_cache_or_clear().clear();
    }

    fn write_include_cache_or_clear(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, HashMap<String, Arc<String>>> {
        match self.inner.include_cache.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                guard
            }
        }
    }

    fn write_component_cache_or_clear(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, HashMap<String, Arc<ComponentTemplate>>> {
        match self.inner.component_ast.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard.clear();
                guard
            }
        }
    }

    fn validate_component_references(&self, nodes: &[TemplateNode]) -> crate::error::Result<()> {
        let mut missing = Vec::new();
        collect_missing_components(nodes, &self.inner.components, &mut missing);
        if let Some(name) = missing.pop() {
            return Err(BlazeError::render(name, "component not registered"));
        }
        Ok(())
    }

    fn write_cache_or_clear(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, HashMap<PathBuf, (Vec<TemplateNode>, usize)>> {
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

impl ComponentResolver for BlazeTemplate {
    fn resolve_component(&self, name: &str) -> crate::error::Result<Arc<ComponentTemplate>> {
        self.get_component_template(name)
    }

    fn resolve_include(&self, path: &str) -> crate::error::Result<Arc<String>> {
        self.get_include(path)
    }
}

fn validate_component_name(name: &str) -> crate::error::Result<()> {
    if !is_valid_component_name(name) {
        return Err(BlazeError::render(
            name,
            "component names must start with an uppercase ASCII letter and contain only ASCII letters, numbers, or underscores",
        ));
    }

    if is_reserved_component_name(name) {
        return Err(BlazeError::render(name, "component tag name is reserved"));
    }

    Ok(())
}

fn is_valid_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_reserved_component_name(name: &str) -> bool {
    matches!(
        name,
        "Each" | "Icon" | "If" | "Image" | "Include" | "Preload" | "Script" | "Style" | "Slot"
    )
}

fn collect_missing_components(
    nodes: &[TemplateNode],
    registry: &HashMap<String, PathBuf>,
    missing: &mut Vec<String>,
) {
    for node in nodes {
        match node {
            TemplateNode::Component(component) => {
                if !registry.contains_key(&component.name) {
                    missing.push(component.name.clone());
                }
                collect_missing_components(&component.children, registry, missing);
            }
            TemplateNode::Each(each) => {
                collect_missing_components(&each.children, registry, missing)
            }
            TemplateNode::If(if_node) => {
                collect_missing_components(&if_node.children, registry, missing)
            }
            _ => {}
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
    fn builder_registers_multiple_components_from_paths() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .register_components([
                ("Person", Path::new("components/Person.html")),
                ("Button", Path::new("components/button.html")),
            ])
            .unwrap()
            .build();

        assert_eq!(
            blaze.component_path("Person").unwrap(),
            Path::new("components/Person.html")
        );
        assert_eq!(
            blaze.component_path("Button").unwrap(),
            Path::new("components/button.html")
        );
        blaze
            .render_page(
                "pages/bench_component.html",
                &json!({ "data": { "age": 30 } }),
            )
            .unwrap();
        blaze
            .render_page_static("pages/bench_component_simple.html")
            .unwrap();
    }

    #[test]
    fn builder_component_registration_accepts_string_paths() {
        let blaze = BlazeTemplate::builder()
            .register_components([("Card", "components/card.html")])
            .unwrap()
            .build();

        assert_eq!(
            blaze.component_path("Card").unwrap(),
            Path::new("components/card.html")
        );
    }

    #[test]
    fn builder_rejects_invalid_and_reserved_component_names() {
        let invalid = BlazeTemplate::builder()
            .register_components([("card", Path::new("components/card.html"))])
            .unwrap_err();
        assert!(invalid.to_string().contains("must start with an uppercase"));

        for name in [
            "Each", "Icon", "If", "Image", "Include", "Preload", "Script", "Style", "Slot",
        ] {
            let reserved = BlazeTemplate::builder()
                .register_components([(name, Path::new("components/reserved.html"))])
                .unwrap_err();
            assert!(
                reserved.to_string().contains("tag name is reserved"),
                "{name} should be reserved"
            );
        }
    }

    #[test]
    fn later_duplicate_component_registration_wins() {
        let blaze = BlazeTemplate::builder()
            .register_components([
                ("Card", Path::new("components/old.html")),
                ("Card", Path::new("components/card.html")),
            ])
            .unwrap()
            .build();

        assert_eq!(
            blaze.component_path("Card").unwrap(),
            Path::new("components/card.html")
        );
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
            .compile_page_templates([Path::new("pages/test_engine_read.html")])
            .unwrap();

        let cache_len = blaze2.inner.ast_nodes.read().unwrap().len();
        assert_eq!(cache_len, 1);
    }

    #[cfg(feature = "cache-bust")]
    mod render_with_hash {
        use super::*;
        use indoc::formatdoc;

        const JS_HASH: &str = "44f4b32954b6da985de1cfd924eacdda";
        const CSS_HASH: &str = "5c7ead8de806c5ed42f44b22c63183ee";

        #[test]
        fn fetches_template_and_passes_to_build() {
            let blaze = BlazeTemplate::builder()
                .template_root_dir("test_files")
                .build();
            let result = blaze
                .render_page_static("pages/test_engine_read.html")
                .unwrap();

            let expected = formatdoc! {r#"
                <script src="/test_files/pages/test_engine_read.js?v={JS_HASH}"></script>
                <link rel="stylesheet" href="/test_files/pages/test_engine_read.css?v={CSS_HASH}">
                <div>Hello World</div>
            "#};
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn render_page_with_include_splices_raw_contents() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        let result = blaze.render_page_static("pages/with_include.html").unwrap();

        // raw CSS is spliced in
        assert!(result.contains("body { color: blue; }"), "got: {result}");
        // verbatim: @ and tag-like text inside the partial are NOT processed
        assert!(result.contains("@notavar"), "got: {result}");
        assert!(result.contains("<Style>"), "got: {result}");
        // surrounding literal markup is preserved
        assert!(result.starts_with("<style>"), "got: {result}");
    }

    #[test]
    fn render_page_with_missing_include_errors() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        let err = blaze
            .render_page_static("pages/with_missing_include.html")
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("reading include"), "got: {msg}");
        assert!(msg.contains("does_not_exist.css"), "got: {msg}");
    }

    #[test]
    fn include_contents_are_cached() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        blaze.render_page_static("pages/with_include.html").unwrap();
        assert_eq!(blaze.inner.include_cache.read().unwrap().len(), 1);
    }

    #[test]
    fn dev_mode_skips_include_cache() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .dev(true)
            .build();
        blaze.render_page_static("pages/with_include.html").unwrap();
        assert_eq!(blaze.inner.include_cache.read().unwrap().len(), 0);
    }

    #[test]
    fn compile_pages_caches_multiple_asts_from_paths() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        blaze
            .compile_page_templates([
                Path::new("pages/test_engine_read.html"),
                Path::new("pages/bench_plain.html"),
            ])
            .unwrap();

        let cache = blaze.inner.ast_nodes.read().unwrap();
        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key(Path::new("pages/test_engine_read.html")));
        assert!(cache.contains_key(Path::new("pages/bench_plain.html")));
    }

    #[test]
    fn compile_pages_accepts_string_paths() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        blaze
            .compile_page_templates(["pages/test_engine_read.html"])
            .unwrap();

        assert!(
            blaze
                .inner
                .ast_nodes
                .read()
                .unwrap()
                .contains_key(Path::new("pages/test_engine_read.html"))
        );
    }

    #[test]
    fn compile_pages_keeps_successful_cache_entries_before_error() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        let error = blaze
            .compile_page_templates([
                Path::new("pages/test_engine_read.html"),
                Path::new("pages/does_not_exist.html"),
                Path::new("pages/bench_plain.html"),
            ])
            .unwrap_err();

        assert!(error.to_string().contains("does_not_exist.html"));
        let cache = blaze.inner.ast_nodes.read().unwrap();
        assert_eq!(cache.len(), 1);
        assert!(cache.contains_key(Path::new("pages/test_engine_read.html")));
        assert!(!cache.contains_key(Path::new("pages/bench_plain.html")));
    }

    #[test]
    fn render_page_populates_cache() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();
        // First render should populate cache
        blaze
            .render_page_static("pages/test_engine_read.html")
            .unwrap();
        assert_eq!(blaze.inner.ast_nodes.read().unwrap().len(), 1);
    }

    #[test]
    fn dev_mode_skips_caching() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .dev(true)
            .build();

        blaze
            .render_page_static("pages/test_engine_read.html")
            .unwrap();
        assert_eq!(blaze.inner.ast_nodes.read().unwrap().len(), 0);
    }

    #[test]
    fn dev_mode_skips_component_caching() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .register_components([("Button", Path::new("components/button.html"))])
            .unwrap()
            .dev(true)
            .build();

        blaze
            .render_page_static("pages/bench_component_simple.html")
            .unwrap();
        assert_eq!(blaze.inner.component_ast.read().unwrap().len(), 0);
    }

    #[test]
    fn render_page_static_matches_render_page_with_empty_data() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();

        let with_data = blaze
            .render_page("pages/test_engine_read.html", &json!({}))
            .unwrap();
        let without_data = blaze
            .render_page_static("pages/test_engine_read.html")
            .unwrap();

        assert_eq!(with_data, without_data);
    }

    #[test]
    fn render_page_static_errors_on_template_with_variables() {
        let blaze = BlazeTemplate::builder()
            .template_root_dir("test_files")
            .build();

        let err = blaze
            .render_page_static("pages/bench_variables.html")
            .unwrap_err();

        assert!(
            err.to_string().to_lowercase().contains("not found"),
            "got: {err}"
        );
    }
}
