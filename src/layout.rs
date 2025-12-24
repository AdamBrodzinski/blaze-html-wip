#![allow(unused)]
use nom::bytes::complete::{take_till1, take_until};
use nom::character::complete::{anychar, char, space0};
use nom::combinator::{peek, recognize, rest, value};
use nom::multi::many_till;
use nom::sequence::delimited;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, character, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value_v2;

#[derive(Debug)]
enum Part<'a> {
    Content(&'a str),
}

struct LayoutContent<'a> {
    before_layout_slot: &'a str,
    after_layout_slot: &'a str,
}

struct PageParts<'a> {
    before_layout_tag: &'a str,
    inside_layout_tag: &'a str,
    after_layout_tag: &'a str,
}

/// Accept a template and only transform the <layout> section, leaving the inner contents
pub fn transform_layout(
    page_template_str: &str,
    layout_template_str: &str,
    _data: &Value,
) -> Result<String, String> {
    let layout_content = layout_template_str.to_owned();
    // find the <layout> tags and return text before tag, inside tags, after closing tag
    let (_, page_parts) = extract_page_parts(page_template_str).unwrap();
    // load the layout and split content before and after <slot/> tag
    let (_, layout_content) = extract_layout_start_end(layout_content.as_str()).unwrap();

    Ok(format!(
        "{}{}{}{}{}",
        page_parts.before_layout_tag,
        layout_content.before_layout_slot,
        page_parts.inside_layout_tag,
        layout_content.after_layout_slot,
        page_parts.after_layout_tag
    ))
}

/// extract text inside layout tags
fn extract_layout_content(input: &str) -> IResult<&str, &str> {
    delimited(tag("<layout>"), take_until("</layout>"), tag("</layout>")).parse(input)
}

/// Extract prefix, layout content, and suffix from page template
fn extract_page_parts(input: &str) -> IResult<&str, PageParts> {
    let (input, before) = take_until("<layout>").parse(input)?;
    let (input, _) = tag("<layout>").parse(input)?;
    let (input, inner) = take_until("</layout>").parse(input)?;
    let (input, _) = tag("</layout>").parse(input)?;
    let (input, after) = rest(input)?;

    Ok((
        input,
        PageParts {
            before_layout_tag: before,
            inside_layout_tag: inner,
            after_layout_tag: after,
        },
    ))
}

fn extract_layout_start_end(input: &str) -> IResult<&str, LayoutContent> {
    // everything up to (not including) where slot_tag starts
    let (input, start) = recognize(many_till(anychar, peek(slot_tag))).parse(input)?;
    let (input, _) = slot_tag(input)?;
    let (input, end) = rest(input)?;
    Ok((
        input,
        LayoutContent {
            before_layout_slot: start,
            after_layout_slot: end,
        },
    ))
}

fn slot_tag(input: &str) -> IResult<&str, ()> {
    value(
        (),
        (
            char('<'),
            space0,
            tag("slot"),
            space0,
            char('/'),
            space0,
            char('>'),
        ),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_transforms_simple_layout() {
        let data = json!(());
        let page_tmpl = "<layout>Content</layout>";
        let layout_tmpl = "Header <slot /> Footer";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_layout_slot_tag_with_spaces() {
        let data = json!(());
        let page_tmpl = "<layout>Content</layout>";
        let layout_tmpl = "Header <  slot   / > Footer";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_layout_slot_tag_with_no_spaces() {
        let data = json!(());
        let page_tmpl = "<layout>Content</layout>";
        let layout_tmpl = "Header <slot/> Footer";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_nested_layout() {
        let data = json!(());
        let page_tmpl = "<layout><div>Content</div></layout>";
        let layout_tmpl = "<header>H</header><slot /><footer>F</footer>";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "<header>H</header><div>Content</div><footer>F</footer>"
        );
    }

    #[test]
    fn it_transforms_layout_with_outer_text() {
        let data = json!(());
        let page_tmpl = "before<layout><div>Content</div></layout>after";
        let layout_tmpl = "<header>H</header><slot /><footer>F</footer>";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "before<header>H</header><div>Content</div><footer>F</footer>after"
        );
    }

    #[test]
    fn it_loads_layout_template() {
        let data = json!(());
        let page_tmpl = "<layout name='foo'>Content</layout>";
        let layout_tmpl = "Header <slot/> Footer";
        let result = transform_layout(page_tmpl, layout_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }
}
