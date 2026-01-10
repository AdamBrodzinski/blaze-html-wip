pub use crate::engine::BlazeTemplate;
pub use crate::error::{BlazeError, BlazeErrorKind};

/// Parser result type with custom error handling that preserves map_res error messages
pub type VResult<'a, T> = nom::IResult<&'a str, T, BlazeError<&'a str>>;

mod ast;
mod engine;
mod error;
mod parse;
mod shared_parsers;
mod tag_asset;
mod text;
