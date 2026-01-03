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

mod build_template;
mod component;
mod data;
pub mod each;
mod layout;
mod parsers;
mod template;
mod variables;

pub use template::BlazeTemplate;
