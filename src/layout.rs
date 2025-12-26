#![allow(unused)]
use nom::bytes::complete::{take_till1, take_until};
use nom::character::complete::{anychar, char, space0, space1};
use nom::combinator::{opt, peek, recognize, rest, value};
use nom::multi::many_till;
use nom::sequence::delimited;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, character, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value;
use crate::parsers::parse_quoted_value;
use crate::BlazeTemplate;

#[derive(Debug)]
enum Part<'a> {
    Content(&'a str),
}

#[derive(Debug)]
enum LayoutIdentifier<'a> {
    Name(&'a str),
    Path(&'a str),
}

struct LayoutContent<'a> {
    before_layout_slot: &'a str,
    after_layout_slot: &'a str,
}

struct PageParts<'a> {
    before_layout_tag: &'a str,
    inside_layout_tag: &'a str,
    after_layout_tag: &'a str,
    layout_identifier: LayoutIdentifier<'a>,
}

pub fn process_layout(
    ctx: &BlazeTemplate,
    page_template_str: &str,
    _data: &Value,
) -> Result<String, String> {
    // If no Layout tag, return input unchanged
    if !page_template_str.contains("<Layout") {
        return Ok(page_template_str.to_string());
    }

    // Parse the page template - errors if Layout tag is malformed
    let (_, page_parts) = extract_page_parts(page_template_str)
        .map_err(|e| format!("Failed to parse page template: {e}"))?;

    // read layout template based on identifier type
    let layout_content_str = match &page_parts.layout_identifier {
        LayoutIdentifier::Name(name) => {
            panic!("Layout name attribute not yet supported: {}", name);
        }
        LayoutIdentifier::Path(path) => ctx.read_template(path)?,
    };

    // load the layout and split content before and after <slot/> tag
    let (_, layout_content) = extract_layout_start_end(&layout_content_str)
        .map_err(|e| format!("Failed to parse layout template: {e}"))?;

    Ok(format!(
        "{}{}{}{}{}",
        page_parts.before_layout_tag,
        layout_content.before_layout_slot,
        page_parts.inside_layout_tag,
        layout_content.after_layout_slot,
        page_parts.after_layout_tag
    ))
}

fn parse_name_attribute(input: &str) -> IResult<&str, LayoutIdentifier> {
    let (input, _) = tag("name").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_quoted_value(input)?;
    Ok((input, LayoutIdentifier::Name(value)))
}

fn parse_path_attribute(input: &str) -> IResult<&str, LayoutIdentifier> {
    let (input, _) = tag("path").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_quoted_value(input)?;
    Ok((input, LayoutIdentifier::Path(value)))
}

// <Layout name='value'> or <Layout path='value'> opening tag
fn parse_layout_opening_tag(input: &str) -> IResult<&str, LayoutIdentifier> {
    let (input, _) = tag("<Layout").parse(input)?;
    let (input, _) = space1(input)?;
    let (input, identifier) = alt((parse_name_attribute, parse_path_attribute)).parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('>').parse(input)?;
    Ok((input, identifier))
}

fn extract_page_parts(input: &str) -> IResult<&str, PageParts> {
    let (input, before) = take_until("<Layout").parse(input)?;
    let (input, layout_identifier) = parse_layout_opening_tag(input)?;
    let (input, inner) = take_until("</Layout>").parse(input)?;
    let (input, _) = tag("</Layout>").parse(input)?;
    let (input, after) = rest(input)?;

    Ok((
        input,
        PageParts {
            before_layout_tag: before,
            inside_layout_tag: inner,
            after_layout_tag: after,
            layout_identifier,
        },
    ))
}

// parse a layout template and handle slot
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
        (char('<'), space0, tag("slot"), space0, char('/'), char('>')),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlazeTemplate;
    use serde_json::json;

    fn setup_ctx() -> BlazeTemplate {
        BlazeTemplate::new().set_root_directory("test_files")
    }

    #[test]
    fn it_transforms_simple_layout() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path='layouts/test.html'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_layout_slot_tag_with_spaces() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path='layouts/test-spaces.html'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_layout_slot_tag_with_no_spaces() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path='layouts/test.html'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_nested_layout() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path='layouts/test-nested.html'><div>Content</div></Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "<header>H</header><div>Content</div><footer>F</footer>"
        );
    }

    #[test]
    fn it_transforms_layout_with_outer_text() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl =
            "before<Layout path='layouts/test-nested.html'><div>Content</div></Layout>after";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "before<header>H</header><div>Content</div><footer>F</footer>after"
        );
    }

    #[test]
    fn it_loads_layout_template_from_path() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path='layouts/test.html'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_parses_layout_path_with_double_quotes() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path=\"layouts/test.html\">Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_errors_when_attribute_is_missing() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_parses_layout_path_with_spaces_around_equals() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout path = 'layouts/test.html'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    #[should_panic(expected = "Layout name attribute not yet supported")]
    fn it_panics_when_name_attribute_is_used() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout name='test'>Content</Layout>";
        let _ = process_layout(&ctx, page_tmpl, &data);
    }

    #[test]
    fn it_errors_when_both_name_and_path_are_present() {
        let ctx = setup_ctx();
        let data = json!(());
        let page_tmpl = "<Layout name='foo' path='bar'>Content</Layout>";
        let result = process_layout(&ctx, page_tmpl, &data);
        assert!(result.is_err());
    }
}
