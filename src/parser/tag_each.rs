use nom::error::context;

use crate::parser_error::VResult;

// each tag iterates over a JSON array of Values
pub fn parse_each_tag(input: &str) -> VResult<'_, (char, &str)> {
    context(
        "each tag",
        // TODO
    )
    .parse(input)
}
