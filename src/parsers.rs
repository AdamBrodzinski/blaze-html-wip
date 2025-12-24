#![allow(unused)]

use nom::character::complete::char;
use nom::{branch::alt, bytes::complete::take_till1, sequence::delimited};

use nom::Parser;

/// Parse quoted attribute value (supports both single and double quotes)
pub fn parse_quoted_value(input: &str) -> nom::IResult<&str, &str> {
    alt((
        delimited(char('\''), take_till1(|c| c == '\''), char('\'')),
        delimited(char('"'), take_till1(|c| c == '"'), char('"')),
    ))
    .parse(input)
}
