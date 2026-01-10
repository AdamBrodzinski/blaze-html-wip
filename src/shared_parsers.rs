#![allow(unused)]
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1, take_while1};
use nom::character::complete::char;
use nom::combinator::cut;
use nom::error::context;
use nom::sequence::{delimited, preceded, separated_pair, terminated};

use crate::VResult;

pub mod attrs {
    use super::*;

    /// parse a pair of single or double quotes and extract inner
    /// Once we see an opening quote, we MUST have a closing quote
    pub fn parse_quoted_value(input: &str) -> VResult<'_, &str> {
        context(
            "quoted value",
            alt((
                preceded(
                    char('\''),
                    cut(terminated(take_till1(|c| c == '\''), char('\''))),
                ),
                preceded(
                    char('"'),
                    cut(context(
                        "missing closing quote",
                        terminated(take_till1(|c| c == '"'), char('"')),
                    )),
                ),
            )),
        )
        .parse(input)
    }

    pub fn parse_attr(input: &str) -> VResult<'_, (&str, &str)> {
        context(
            "attribute",
            separated_pair(take_while1(is_attr_name_char), tag("="), parse_quoted_value),
        )
        .parse(input)
    }

    fn is_attr_name_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    }
}
