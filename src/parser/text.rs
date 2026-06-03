use nom::error::{ErrorKind, ParseError};

use crate::ast::TemplateNode;

use super::error::{BlazeParseError, VResult};

pub fn parse_text(input: &str) -> VResult<'_, TemplateNode> {
    let idx = find_special_start(input).unwrap_or(input.len());
    if idx == 0 {
        return Err(nom::Err::Error(BlazeParseError::from_error_kind(
            input,
            ErrorKind::TakeUntil,
        )));
    }
    let (text, remaining) = input.split_at(idx);
    Ok((remaining, TemplateNode::Text(text.to_string())))
}

fn find_special_start(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            return Some(i);
        }
        if bytes[i] == b'<' {
            let rest = &input[i..];
            if rest.starts_with("<Each")
                || rest.starts_with("<If")
                || rest.starts_with("<Include")
                || rest.starts_with("<Script")
                || rest.starts_with("<Style")
                || rest.starts_with("<Slot")
            {
                return Some(i);
            }
            if rest
                .as_bytes()
                .get(1)
                .copied()
                .map(|b| b.is_ascii_uppercase())
                == Some(true)
            {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}
