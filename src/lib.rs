pub use crate::engine::BlazeTemplate;
pub use crate::parser_error::{BlazeParseError, BlazeParseErrorKind};

/// Parser result type with custom error handling that preserves map_res error messages
pub type VResult<'a, T> = nom::IResult<&'a str, T, BlazeParseError<&'a str>>;

mod ast;
mod engine;
mod parser_error;
mod parse;
mod shared_parsers;
mod tag_asset;
mod text;
