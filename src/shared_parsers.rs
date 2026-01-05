#![allow(unused)]
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_while1};
use nom::character::complete::char;
use nom::sequence::{delimited, separated_pair};
use nom::{IResult, Parser};

pub mod attrs {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Attr<'a> {
        name: &'a str,
        value: &'a str,
    }

    /// parse an attr with single or double quotes
    pub fn parse_quoted_value(input: &str) -> nom::IResult<&str, &str> {
        alt((
            delimited(char('\''), take_till1(|c| c == '\''), char('\'')),
            delimited(char('"'), take_till1(|c| c == '"'), char('"')),
        ))
        .parse(input)
    }

    pub fn parse_attr(input: &str) -> IResult<&str, Attr<'_>> {
        let (input, (name, value)) =
            separated_pair(take_while1(is_attr_name_char), tag("="), parse_quoted_value)
                .parse(input)?;

        Ok((input, Attr { name, value }))
    }

    fn is_attr_name_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    }
}
