//! Custom error types for blaze-html template parsing
//!
//! Extends nom's error handling to preserve error messages from `map_res` closures
//! via the `External` variant in `BlazeParseErrorKind`.

use nom::error::{ContextError, ErrorKind, FromExternalError, ParseError};
use std::fmt;

/// Parser result type with custom error handling that preserves map_res error messages
pub type VResult<'a, T> = nom::IResult<&'a str, T, BlazeParseError<&'a str>>;

/// Error kind variants for template parsing errors
#[derive(Debug, Clone, PartialEq)]
pub enum BlazeParseErrorKind {
    /// Static context string added via `context()` combinator
    Context(&'static str),
    /// Expected character from `char()` combinator
    Char(char),
    /// Standard nom error kind
    Nom(ErrorKind),
    /// External error message from `map_res` closure - preserves the actual error string
    External(String),
}

/// Accumulating error type that tracks parsing failures with positions
#[derive(Debug, Clone, PartialEq)]
pub struct BlazeParseError<I> {
    /// List of errors with their input positions, most recent last
    pub errors: Vec<(I, BlazeParseErrorKind)>,
}

impl<I> ParseError<I> for BlazeParseError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        BlazeParseError {
            errors: vec![(input, BlazeParseErrorKind::Nom(kind))],
        }
    }

    fn append(input: I, kind: ErrorKind, mut other: Self) -> Self {
        other.errors.push((input, BlazeParseErrorKind::Nom(kind)));
        other
    }
}

impl<I> ContextError<I> for BlazeParseError<I> {
    fn add_context(input: I, ctx: &'static str, mut other: Self) -> Self {
        other.errors.push((input, BlazeParseErrorKind::Context(ctx)));
        other
    }
}

impl<I> FromExternalError<I, String> for BlazeParseError<I> {
    fn from_external_error(input: I, _kind: ErrorKind, e: String) -> Self {
        BlazeParseError {
            errors: vec![(input, BlazeParseErrorKind::External(e))],
        }
    }
}

impl<I: fmt::Display> fmt::Display for BlazeParseError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Parse error:")?;
        for (input, kind) in &self.errors {
            match kind {
                BlazeParseErrorKind::Context(ctx) => writeln!(f, "  in {ctx} at: {input}")?,
                BlazeParseErrorKind::Char(c) => writeln!(f, "  expected '{c}' at: {input}")?,
                BlazeParseErrorKind::Nom(ek) => writeln!(f, "  {:?} at: {input}", ek)?,
                BlazeParseErrorKind::External(msg) => writeln!(f, "  {msg}")?,
            }
        }
        Ok(())
    }
}

impl<I: fmt::Debug + fmt::Display> std::error::Error for BlazeParseError<I> {}

/// Create a nom Failure error from a string message
pub fn make_error(input: &str, msg: impl Into<String>) -> nom::Err<BlazeParseError<&str>> {
    nom::Err::Failure(BlazeParseError::from_external_error(
        input,
        ErrorKind::Fail,
        msg.into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_external_error_preserves_message() {
        let input = "test input";
        let error: BlazeParseError<&str> = BlazeParseError::from_external_error(
            input,
            ErrorKind::MapRes,
            "custom error".to_string(),
        );

        assert_eq!(error.errors.len(), 1);
        assert_eq!(
            error.errors[0],
            (
                input,
                BlazeParseErrorKind::External("custom error".to_string())
            )
        );
    }
}
