#![allow(unused)]
use nom::bytes::complete::take_till1;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value_v2;

#[derive(Debug)]
enum Part<'a> {
    Text(&'a str),
    Var(&'a str),
}

/// Accept a template and only transform the <layout> section, leaving the inner contents
pub fn transform_layout(
    page_template_str: &str,
    layout_template_str: &str,
    _data: &Value,
) -> Result<String, String> {
    let output = page_template_str.to_string();

    Ok(output.to_string())
}

/*

page_template_str:
<layout>Middle</layout>

layout_template_str: (would be read from disk in future, passed in now)
Before <slot/> After

let new_page_str = transform_layout(layout_str, page_str)


TextBefore(&str)
Slot
TextAfter(&str)

"Before Middle After"

let textbefore = takeuntiltag0(|tag|  tag == "<include-script")
let slot = combo(tag("<slot"), emptyspace0, preceeded(space0, tag("/>")))
let textafter = take0()

let new_page_str = transform_layout(layout_str, page_str)

*/

// - parse page template into

// // ---------------------- variable ----------------------
//
// /// a valid variable name (after @)
// fn variable_key(input: &str) -> IResult<&str, &str> {
//     take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.').parse(input)
// }
//
// /// parse the entire variable @foo and return the variable name foo
// fn variable(input: &str) -> IResult<&str, Part> {
//     // preceeded matches the @ + var_name then discards @ tag
//     let (input, name) = preceded(tag("@"), variable_key).parse(input)?;
//     Ok((input, Part::Var(name)))
// }
//
// // ---------------------- text ----------------------
//
// fn text(input: &str) -> IResult<&str, Part> {
//     let (input, txt) = take_till1(|c| c == '@').parse(input)?;
//     Ok((input, Part::Text(txt)))
// }
//
// // ---------------------- template ----------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_variables() {
        let data = json!(());
        let page_tmpl = "<layout>Content</layout>";
        let layout_tmpl = "Header <slot/> Footer";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }
}
