#![warn(missing_docs)]

//! A component-driven server-side HTML template engine.
//!
//! Templates are loaded at runtime and use `@name` for variable interpolation.
//! Variables are HTML-escaped by default; `@!name` emits trusted content without
//! escaping.
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
