pub use crate::engine::BlazeTemplate;

use nom_language::error::VerboseError;

/// Parser result type with verbose error information
pub type VResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

mod ast;
mod engine;
mod parse;
mod shared_parsers;
mod tag_asset;
mod text;
