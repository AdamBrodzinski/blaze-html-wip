use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use crate::build_template::build_template;

#[derive(Debug, Clone)]
pub struct BlazeTemplateConfig {
    pub dev: bool,
    pub panic_on_error: bool,
    pub panic_on_null: bool,
    pub project_path: String,
    pub root_dir: String,
    pub cache_file_read: bool,
}

#[derive(Debug, Clone)]
pub struct BlazeTemplateBuilder {
    dev: bool,
    panic_on_error: bool,
    panic_on_null: bool,
    project_path: String,
    root_dir: String,
    cache_file_read: bool,
}

impl Default for BlazeTemplateBuilder {
    fn default() -> Self {
        Self {
            dev: false,
            panic_on_error: false,
            panic_on_null: true,
            project_path: env!("CARGO_MANIFEST_DIR").to_string(),
            root_dir: "src".to_string(),
            cache_file_read: true,
        }
    }
}

impl BlazeTemplateBuilder {
    pub fn set_root_directory(mut self, path: &str) -> Self {
        self.root_dir = path.to_string();
        self
    }

    pub fn panic_on_error(mut self, should_panic: bool) -> Self {
        self.panic_on_error = should_panic;
        self
    }

    pub fn panic_on_null(mut self, should_panic: bool) -> Self {
        self.panic_on_null = should_panic;
        self
    }

    pub fn enable_dev(mut self, dev_enabled: bool) -> Self {
        self.dev = dev_enabled;
        self
    }

    pub fn cache_file_read(mut self, enabled: bool) -> Self {
        self.cache_file_read = enabled;
        self
    }

    pub fn build(self) -> BlazeTemplate {
        BlazeTemplate {
            config: BlazeTemplateConfig {
                dev: self.dev,
                panic_on_error: self.panic_on_error,
                panic_on_null: self.panic_on_null,
                project_path: self.project_path,
                root_dir: self.root_dir,
                cache_file_read: self.cache_file_read,
            },
            file_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlazeTemplate {
    config: BlazeTemplateConfig,
    file_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl BlazeTemplate {
    pub fn builder() -> BlazeTemplateBuilder {
        BlazeTemplateBuilder::default()
    }

    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn config(&self) -> &BlazeTemplateConfig {
        &self.config
    }

    fn get_template_path(&self, rel_page_path: &str) -> PathBuf {
        [
            &self.config.project_path,
            &self.config.root_dir,
            rel_page_path,
        ]
        .iter()
        .collect()
    }

    pub fn read_template(&self, rel_page_path: &str) -> Result<String, String> {
        let use_cache = self.config.cache_file_read && !self.config.dev;

        if use_cache {
            let cache = self
                .file_cache
                .read()
                .map_err(|e| format!("Cache read lock poisoned: {e}"))?;
            if let Some(content) = cache.get(rel_page_path) {
                return Ok(content.clone());
            }
        }

        let template_path = self.get_template_path(rel_page_path);
        let content = std::fs::read_to_string(&template_path).map_err(|e| e.to_string())?;

        if use_cache {
            self.file_cache
                .write()
                .map_err(|e| format!("Cache write lock poisoned: {e}"))?
                .entry(rel_page_path.to_string())
                .or_insert(content.clone());
        }

        Ok(content)
    }

    pub fn render_page(&self, rel_page_path: &str, data: &Value) -> Result<String, String> {
        let template_file = self.read_template(rel_page_path)?;
        build_template(self, &template_file, data)
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
        assert_eq!(blaze.config().root_dir, "src");
        assert_eq!(blaze.config().dev, false);
        assert_eq!(blaze.config().panic_on_error, false);
        assert_eq!(blaze.config().panic_on_null, true);
    }

    #[test]
    fn test_builder_pattern() {
        let blaze = BlazeTemplate::builder()
            .set_root_directory("customer/pages")
            .enable_dev(true)
            .panic_on_error(true)
            .panic_on_null(false)
            .build();

        assert_eq!(blaze.config().root_dir, "customer/pages");
        assert_eq!(blaze.config().dev, true);
        assert_eq!(blaze.config().panic_on_error, true);
        assert_eq!(blaze.config().panic_on_null, false);
    }

    #[test]
    fn fetches_template_and_passes_to_build() {
        let blaze = BlazeTemplate::builder()
            .set_root_directory("test_files")
            .build();
        let data = json!(());
        let result = blaze.render_page("pages/static.html", &data).unwrap();
        assert_eq!(result, "<div>Hello World</div>\n");
    }
}
