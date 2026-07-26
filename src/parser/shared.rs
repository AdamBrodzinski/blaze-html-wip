#![allow(unused)]
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_while1};
use nom::character::complete::char;
use nom::combinator::cut;
use nom::error::context;
use nom::sequence::{delimited, preceded, separated_pair, terminated};

use super::error::VResult;

pub mod attrs {
    use super::*;

    /// parse a pair of single or double quotes and extract inner
    /// Once we see an opening quote, we MUST have a closing quote
    /// Returns (quote_char, inner_value)
    pub fn parse_quoted_value(input: &str) -> VResult<'_, (char, &str)> {
        context(
            "quoted value",
            alt((
                preceded(
                    char('\''),
                    cut(terminated(take_till(|c| c == '\''), char('\''))),
                )
                .map(|v| ('\'', v)),
                preceded(
                    char('"'),
                    cut(context(
                        "missing closing quote",
                        terminated(take_till(|c| c == '"'), char('"')),
                    )),
                )
                .map(|v| ('"', v)),
            )),
        )
        .parse(input)
    }

    /// Returns (attr_name, attr_value, quote_char)
    pub fn parse_attr(input: &str) -> VResult<'_, (&str, &str, char)> {
        context(
            "attribute",
            separated_pair(take_while1(is_attr_name_char), tag("="), parse_quoted_value)
                .map(|(name, (quote, value))| (name, value, quote)),
        )
        .parse(input)
    }

    fn is_attr_name_char(c: char) -> bool {
        // todo should this start with alpha only?
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::attrs::parse_attr;

    #[test]
    fn parses_empty_double_quoted_attr() {
        let (remaining, (name, value, quote)) = parse_attr(r#"title="" rest"#).unwrap();

        assert_eq!(remaining, " rest");
        assert_eq!((name, value, quote), ("title", "", '"'));
    }

    #[test]
    fn parses_empty_single_quoted_attr() {
        let (remaining, (name, value, quote)) = parse_attr("title='' rest").unwrap();

        assert_eq!(remaining, " rest");
        assert_eq!((name, value, quote), ("title", "", '\''));
    }
}
