//! Custom error types for blaze-html template parsing
//!
//! Extends nom's error handling to preserve error messages from `map_res` closures
//! via the `External` variant in `BlazeErrorKind`.

use nom::error::{ContextError, ErrorKind, FromExternalError, ParseError};
use std::fmt;

/// Error kind variants for template parsing errors
#[derive(Debug, Clone, PartialEq)]
pub enum BlazeErrorKind {
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
pub struct BlazeError<I> {
    /// List of errors with their input positions, most recent last
    pub errors: Vec<(I, BlazeErrorKind)>,
}

impl<I> ParseError<I> for BlazeError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        BlazeError {
            errors: vec![(input, BlazeErrorKind::Nom(kind))],
        }
    }

    fn append(input: I, kind: ErrorKind, mut other: Self) -> Self {
        other.errors.push((input, BlazeErrorKind::Nom(kind)));
        other
    }
}

impl<I> ContextError<I> for BlazeError<I> {
    fn add_context(input: I, ctx: &'static str, mut other: Self) -> Self {
        other.errors.push((input, BlazeErrorKind::Context(ctx)));
        other
    }
}

impl<I> FromExternalError<I, String> for BlazeError<I> {
    fn from_external_error(input: I, _kind: ErrorKind, e: String) -> Self {
        BlazeError {
            errors: vec![(input, BlazeErrorKind::External(e))],
        }
    }
}

impl<I: fmt::Display> fmt::Display for BlazeError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Parse error:")?;
        for (input, kind) in &self.errors {
            match kind {
                BlazeErrorKind::Context(ctx) => writeln!(f, "  in {ctx} at: {input}")?,
                BlazeErrorKind::Char(c) => writeln!(f, "  expected '{c}' at: {input}")?,
                BlazeErrorKind::Nom(ek) => writeln!(f, "  {:?} at: {input}", ek)?,
                BlazeErrorKind::External(msg) => writeln!(f, "  {msg}")?,
            }
        }
        Ok(())
    }
}

impl<I: fmt::Debug + fmt::Display> std::error::Error for BlazeError<I> {}

/// Create a nom Failure error from a string message
pub fn make_error(input: &str, msg: String) -> nom::Err<BlazeError<&str>> {
    nom::Err::Failure(BlazeError::from_external_error(input, ErrorKind::Fail, msg))
}

/// Debug function to see all errors in the chain
#[allow(dead_code)]
pub fn debug_error(e: &BlazeError<&str>) {
    eprintln!("=== Error chain ({} errors) ===", e.errors.len());
    for (i, (pos, kind)) in e.errors.iter().enumerate() {
        let preview: String = pos.chars().take(30).collect();
        eprintln!("{i}: {kind:?} at \"{preview}...\"");
    }
    eprintln!("=== End error chain ===\n");
}

/// Convert a BlazeError into a human-readable error message with line/column info
///
/// Shows the most useful error (External or Char) prominently, with context chain below.
/// Filters out internal nom errors (like Tag, MapRes) that aren't helpful to users.
pub fn convert_error(input: &str, e: BlazeError<&str>) -> String {
    // Find the primary error - prefer External, then Char, then first Context
    let primary = e
        .errors
        .iter()
        .find(|(_, k)| matches!(k, BlazeErrorKind::External(_)))
        .or_else(|| {
            e.errors
                .iter()
                .find(|(_, k)| matches!(k, BlazeErrorKind::Char(_)))
        })
        .or_else(|| {
            e.errors
                .iter()
                .find(|(_, k)| matches!(k, BlazeErrorKind::Context(_)))
        });

    let Some((substring, kind)) = primary else {
        return "Unknown parse error".to_string();
    };

    let mut result = String::new();
    let offset = input.len() - substring.len();

    // Handle empty input case
    if input.is_empty() {
        let msg = match kind {
            BlazeErrorKind::External(msg) => format!("Error: {msg}"),
            BlazeErrorKind::Char(c) => format!("Error: expected '{c}', got end of input"),
            BlazeErrorKind::Context(ctx) => format!("Error in {ctx}: unexpected end of input"),
            BlazeErrorKind::Nom(ek) => format!("Error: {ek:?}"),
        };
        return msg;
    }

    // Calculate position info
    let line_num = input[..offset].chars().filter(|&c| c == '\n').count() + 1;
    let line_start = input[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line_end = substring.find('\n').unwrap_or(substring.len());
    let line = &input[line_start..offset + line_end];
    let column = offset - line_start;

    // Format the primary error message
    let error_msg = match kind {
        BlazeErrorKind::External(msg) => msg.clone(),
        BlazeErrorKind::Char(c) => {
            if substring.is_empty() {
                format!("expected '{c}', got end of input")
            } else {
                format!(
                    "expected '{c}', found '{}'",
                    substring.chars().next().unwrap()
                )
            }
        }
        BlazeErrorKind::Context(ctx) => ctx.to_string(),
        BlazeErrorKind::Nom(ek) => format!("{ek:?}"),
    };

    result.push_str(&format!("Error at line {line_num}: {error_msg}\n\n"));
    result.push_str(&format!("  {line}\n"));
    result.push_str(&format!("  {}^\n", " ".repeat(column)));

    // Collect context chain (skip Nom errors, they're internal details)
    let contexts: Vec<&str> = e
        .errors
        .iter()
        .rev() // outermost first
        .filter_map(|(_, k)| match k {
            BlazeErrorKind::Context(ctx) => Some(*ctx),
            _ => None,
        })
        .collect();

    if !contexts.is_empty() {
        result.push_str(&format!("\nContext: {}\n", contexts.join(" → ")));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_external_error_preserves_message() {
        let input = "test input";
        let error: BlazeError<&str> =
            BlazeError::from_external_error(input, ErrorKind::MapRes, "custom error".to_string());

        assert_eq!(error.errors.len(), 1);
        assert_eq!(
            error.errors[0],
            (input, BlazeErrorKind::External("custom error".to_string()))
        );
    }

    #[test]
    fn convert_error_shows_external_message() {
        let input = "<Script foo=\"bar\" />";
        let error = BlazeError {
            errors: vec![(
                &input[8..], // position after "<Script "
                BlazeErrorKind::External("path is a required attribute".to_string()),
            )],
        };

        let output = convert_error(input, error);
        assert!(output.contains("path is a required attribute"));
        assert!(output.contains("line 1"));
    }
}
