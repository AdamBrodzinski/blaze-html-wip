use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::take_until;
use nom::combinator::{rest, verify};
use nom::error::context;

use crate::parser_error::VResult;
use crate::ast::TemplateNode;

pub fn parse_text(input: &str) -> VResult<'_, TemplateNode> {
    let (remaining, text) = context(
        "text content",
        verify(
            alt((take_until("<Script"), take_until("<Style"), rest)),
            |s: &str| !s.is_empty(),
        ),
    )
    .parse(input)?;

    Ok((remaining, TemplateNode::Text(text.to_string())))
}
