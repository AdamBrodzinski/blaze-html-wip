#![warn(missing_docs)]
//! Fast server-side HTML templating for Rust, designed around component-first ergonomics. Build isolated components, pass data down as props, and keep front-end-style organization without a front-end framework. BlazeHTML pairs naturally with [htmx](https://htmx.org) and [Datastar](https://data-star.dev).
//!
//! - Fast iteration workflow, zero recompiles for HTML, CSS, JS changes
//! - Modern component organization
//! - Logic is written and tested in Rust, minimal markup testing
//! - Markup works with existing html editor tooling
//! - Compose generic components that pass data through as props (lexically scoped)
//! - Variables are automatically escaped
//!
//! # Example
//!
//! ```no_run
//! use blaze_html::BlazeTemplate;
//! use serde_json::json;
//!
//! # fn main() -> blaze_html::Result<()> {
//! let templates = BlazeTemplate::builder()
//!     .template_root_dir("templates")
//!     .build();
//!
//! let html = templates.render_page(
//!     "pages/home.html",
//!     &json!({ "user": { "name": "Ada" } }),
//! )?;
//! # Ok(())
//! # }
//! ```

pub use crate::engine::{BlazeTemplate, BlazeTemplateBuilder};
pub use crate::error::{
    BlazeError, IoErrorDetails, IoOperation, ParseErrorDetails, RenderErrorDetails, Result,
};

mod ast;
mod engine;
mod error;
mod parser;
mod render;
