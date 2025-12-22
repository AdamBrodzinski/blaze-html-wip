#![allow(unused)]
use nom::bytes::complete::{take_till1, take_until};
use nom::sequence::delimited;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value_v2;

#[derive(Debug)]
enum Part<'a> {
    Content(&'a str),
}

struct LayoutContent<'a> {
    start: &'a str,
    end: &'a str,
}

/// Accept a template and only transform the <layout> section, leaving the inner contents
pub fn transform_layout(
    page_template_str: &str,
    layout_template_str: &str,
    _data: &Value,
) -> Result<String, String> {
    let output = page_template_str.to_string();
    let layout_content = layout_template_str.to_owned();
    let (_input, layout_tag_inner) = extract_layout_content(page_template_str).unwrap();
    dbg!(layout_tag_inner);
    let (_, layout_content) = extract_layout_start_end(layout_content.as_str()).unwrap();

    Ok(format!(
        "{}{}{}",
        layout_content.start, layout_tag_inner, layout_content.end
    ))
}

/// extract text inside layout tags
fn extract_layout_content(input: &str) -> IResult<&str, &str> {
    delimited(tag("<layout>"), take_until("</layout>"), tag("</layout>")).parse(input)
}

fn extract_layout_start_end(input: &str) -> IResult<&str, LayoutContent> {
    Ok((
        input,
        LayoutContent {
            start: "START",
            end: "END",
        },
    ))
}

/*

output
"Before Middle After"

let textbefore = takeuntiltag0(|tag|  tag == "<include-script")
let slot = combo(tag("<slot"), emptyspace0, preceeded(space0, tag("/>")))
let textafter = take0()

let new_page_str = transform_layout(layout_str, page_str)

*/

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
