use nom::combinator::verify;
use nom::{IResult, Parser};
use nom::{branch::alt, bytes::complete::take_until, combinator::rest};

use crate::ast::TemplateNode;

pub fn parse_text(input: &str) -> IResult<&str, TemplateNode> {
    let (remaining, text) = verify(
        alt((take_until("<Script"), take_until("<Style"), rest)),
        |s: &str| !s.is_empty(),
    )
    .parse(input)?;

    Ok((remaining, TemplateNode::Text(text.to_string())))
}
