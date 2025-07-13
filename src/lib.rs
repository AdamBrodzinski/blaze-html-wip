//! # Blaze HTML
//!
//! This crate provides another syntax for html templating that works with any
//! HTML formatter/parser. The syntax is similar to web components and tries to
//!
//! ## Features
//! - Variable replacement with context
//!
//!
// ## Example
// ```rust
// use my_crate::do_foo;
// do_foo();
// ```

mod html;
mod variables;
use html::{rewrite_comments, rewrite_component, rewrite_each_tags};
use variables::replace_variables;

pub fn render_template_str(template: &str, data: &serde_json::Value) -> String {
    let template = rewrite_each_tags(template, data);
    let template = rewrite_component(&template, data);
    let template = rewrite_comments(&template);
    replace_variables(&template, data)
}
