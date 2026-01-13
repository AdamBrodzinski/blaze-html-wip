//! Error types for the blaze-html template engine.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::parser_error::{BlazeParseError, BlazeParseErrorKind};

/// Result type for blaze-html operations.
pub type Result<T> = std::result::Result<T, BlazeError>;

/// The main error type for blaze-html template operations.
#[derive(Debug)]
pub enum BlazeError {
    /// Template parsing failed due to syntax error or invalid construct.
    Parse(ParseErrorDetails),

    /// File I/O operation failed.
    Io(IoErrorDetails),
}

/// Detailed information about a template parsing error.
#[derive(Debug, Clone)]
pub struct ParseErrorDetails {
    /// Human-readable error message.
    pub message: String,
    /// Line number where the error occurred (1-indexed).
    pub line: usize,
    /// Column number where the error occurred (1-indexed).
    pub column: usize,
    /// The content of the line containing the error.
    pub line_content: String,
    /// Parser context chain (outermost first).
    pub context: Vec<&'static str>,
}

/// Detailed information about an I/O error.
#[derive(Debug)]
pub struct IoErrorDetails {
    /// The operation that failed.
    pub operation: IoOperation,
    /// The file path involved.
    pub path: PathBuf,
    /// The underlying I/O error.
    pub source: io::Error,
}

/// The type of I/O operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    /// Reading a template file from disk.
    ReadTemplate,
    /// Reading an asset file for cache-busting hash computation.
    HashAsset,
}

// === Display Implementations ===

impl fmt::Display for BlazeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlazeError::Parse(details) => {
                write!(
                    f,
                    "Parse error at line {}:{}: {}",
                    details.line, details.column, details.message
                )?;
                if !details.line_content.is_empty() {
                    write!(f, "\n\n  {}", details.line_content)?;
                    write!(f, "\n  {}^", " ".repeat(details.column.saturating_sub(1)))?;
                }
                if !details.context.is_empty() {
                    write!(f, "\n\nContext: {}", details.context.join(" -> "))?;
                }
                Ok(())
            }
            BlazeError::Io(details) => {
                let op = match details.operation {
                    IoOperation::ReadTemplate => "reading template",
                    IoOperation::HashAsset => "hashing asset",
                };
                write!(
                    f,
                    "I/O error {}: '{}': {}",
                    op,
                    details.path.display(),
                    details.source
                )
            }
        }
    }
}

impl std::error::Error for BlazeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BlazeError::Io(details) => Some(&details.source),
            _ => None,
        }
    }
}

impl fmt::Display for ParseErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at line {}:{}", self.message, self.line, self.column)
    }
}

impl fmt::Display for IoErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} '{}': {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl fmt::Display for IoOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoOperation::ReadTemplate => write!(f, "ReadTemplate"),
            IoOperation::HashAsset => write!(f, "HashAsset"),
        }
    }
}

// === From Implementations ===

impl From<ParseErrorDetails> for BlazeError {
    fn from(details: ParseErrorDetails) -> Self {
        BlazeError::Parse(details)
    }
}

impl From<IoErrorDetails> for BlazeError {
    fn from(details: IoErrorDetails) -> Self {
        BlazeError::Io(details)
    }
}

// === Conversion from Internal Parser Error ===

impl ParseErrorDetails {
    /// Create parse error details from the internal nom-compatible error.
    pub fn from_blaze_parse_error(input: &str, err: BlazeParseError<&str>) -> Self {
        // Find primary error: External > Char > Context > Nom
        let primary = err
            .errors
            .iter()
            .find(|(_, k)| matches!(k, BlazeParseErrorKind::External(_)))
            .or_else(|| {
                err.errors
                    .iter()
                    .find(|(_, k)| matches!(k, BlazeParseErrorKind::Char(_)))
            })
            .or_else(|| {
                err.errors
                    .iter()
                    .find(|(_, k)| matches!(k, BlazeParseErrorKind::Context(_)))
            })
            .or_else(|| err.errors.first());

        let (message, line, column, line_content) = match primary {
            Some((substring, kind)) => {
                let offset = input.len().saturating_sub(substring.len());
                let line = input[..offset].chars().filter(|&c| c == '\n').count() + 1;
                let line_start = input[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end_offset = substring.find('\n').unwrap_or(substring.len());
                let line_content = input
                    .get(line_start..offset + line_end_offset)
                    .unwrap_or("")
                    .to_string();
                let column = offset.saturating_sub(line_start) + 1;

                let message = match kind {
                    BlazeParseErrorKind::External(msg) => msg.clone(),
                    BlazeParseErrorKind::Char(c) => {
                        if substring.is_empty() {
                            format!("expected '{}', found end of input", c)
                        } else {
                            format!(
                                "expected '{}', found '{}'",
                                c,
                                substring.chars().next().unwrap_or('?')
                            )
                        }
                    }
                    BlazeParseErrorKind::Context(ctx) => (*ctx).to_string(),
                    BlazeParseErrorKind::Nom(ek) => format!("{:?}", ek),
                };

                (message, line, column, line_content)
            }
            None => ("Unknown parse error".to_string(), 1, 1, String::new()),
        };

        // Collect context chain (outermost first)
        let context: Vec<&'static str> = err
            .errors
            .iter()
            .rev()
            .filter_map(|(_, k)| match k {
                BlazeParseErrorKind::Context(ctx) => Some(*ctx),
                _ => None,
            })
            .collect();

        ParseErrorDetails {
            message,
            line,
            column,
            line_content,
            context,
        }
    }

    /// Create parse error details at a specific position in the input.
    pub fn at_position(input: &str, position: usize, message: impl Into<String>) -> Self {
        let offset = position.min(input.len());
        let line = input[..offset].chars().filter(|&c| c == '\n').count() + 1;
        let line_start = input[..offset].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = input[offset..]
            .find('\n')
            .map(|p| offset + p)
            .unwrap_or(input.len());
        let line_content = input.get(line_start..line_end).unwrap_or("").to_string();
        let column = offset.saturating_sub(line_start) + 1;

        ParseErrorDetails {
            message: message.into(),
            line,
            column,
            line_content,
            context: vec![],
        }
    }
}

impl BlazeError {
    /// Create a `BlazeError` from a nom error wrapper.
    pub fn from_nom_error(input: &str, err: nom::Err<BlazeParseError<&str>>) -> Self {
        match err {
            nom::Err::Incomplete(_) => BlazeError::Parse(ParseErrorDetails {
                message: "Incomplete input".to_string(),
                line: 1,
                column: 1,
                line_content: String::new(),
                context: vec![],
            }),
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                BlazeError::Parse(ParseErrorDetails::from_blaze_parse_error(input, e))
            }
        }
    }

    /// Create an I/O error for template reading.
    pub fn template_io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        BlazeError::Io(IoErrorDetails {
            operation: IoOperation::ReadTemplate,
            path: path.into(),
            source,
        })
    }

    /// Create an I/O error for asset hashing.
    pub fn asset_io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        BlazeError::Io(IoErrorDetails {
            operation: IoOperation::HashAsset,
            path: path.into(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_parse_error() {
        let err = BlazeError::Parse(ParseErrorDetails {
            message: "expected '>'".to_string(),
            line: 5,
            column: 12,
            line_content: "<Script path=\"foo.js\"".to_string(),
            context: vec!["Script tag", "closing tag"],
        });
        let display = format!("{}", err);
        assert!(display.contains("line 5:12"));
        assert!(display.contains("expected '>'"));
        assert!(display.contains("Script tag -> closing tag"));
    }

    #[test]
    fn display_io_error() {
        let err = BlazeError::Io(IoErrorDetails {
            operation: IoOperation::ReadTemplate,
            path: PathBuf::from("templates/missing.html"),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        });
        let display = format!("{}", err);
        assert!(display.contains("reading template"));
        assert!(display.contains("missing.html"));
    }

    #[test]
    fn error_source_chain() {
        use std::error::Error;

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err = BlazeError::Io(IoErrorDetails {
            operation: IoOperation::ReadTemplate,
            path: PathBuf::from("test.html"),
            source: io_err,
        });

        assert!(err.source().is_some());
    }

    #[test]
    fn at_position_calculates_correctly() {
        let input = "line1\nline2\nline3";
        let details = ParseErrorDetails::at_position(input, 7, "test error");
        assert_eq!(details.line, 2);
        assert_eq!(details.column, 2);
        assert_eq!(details.line_content, "line2");
    }
}
