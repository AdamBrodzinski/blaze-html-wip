#![allow(unused)]
use nom::bytes::complete::{take_till1, take_until};
use nom::character::complete::{anychar, char, space0, space1};
use nom::combinator::{peek, recognize, rest, value};
use nom::multi::many_till;
use nom::sequence::delimited;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, character, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value;
use crate::parsers::parse_quoted_value;

#[derive(Debug)]
enum Part<'a> {
    Content(&'a str),
}

#[derive(Debug)]
enum ComponentIdentifier<'a> {
    Name(&'a str),
    Path(&'a str),
}

struct ComponentContent<'a> {
    before_slot: &'a str,
    after_slot: &'a str,
}

struct PageParts<'a> {
    before_component_tag: &'a str,
    inside_component_tag: &'a str,
    after_component_tag: &'a str,
    component_identifier: ComponentIdentifier<'a>,
}

pub fn transform_component(
    page_template_str: &str,
    component_template_str: &str,
    _data: &Value,
) -> Result<String, String> {
    let component_content = component_template_str.to_owned();
    // find the <Component> tags and return text before tag, inside tags, after closing tag
    let (_, page_parts) = extract_page_parts(page_template_str)
        .map_err(|e| format!("Failed to parse page template: {e}"))?;
    // load the component and split content before and after <slot/> tag
    let (_, component_content) = extract_component_start_end(component_content.as_str())
        .map_err(|e| format!("Failed to parse component template: {e}"))?;

    Ok(format!(
        "{}{}{}{}{}",
        page_parts.before_component_tag,
        component_content.before_slot,
        page_parts.inside_component_tag,
        component_content.after_slot,
        page_parts.after_component_tag
    ))
}

/// Parse name='value' attribute and return ComponentIdentifier::Name
fn parse_name_attribute(input: &str) -> IResult<&str, ComponentIdentifier> {
    let (input, _) = tag("name").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_quoted_value(input)?;
    Ok((input, ComponentIdentifier::Name(value)))
}

/// Parse path='value' attribute and return ComponentIdentifier::Path
fn parse_path_attribute(input: &str) -> IResult<&str, ComponentIdentifier> {
    let (input, _) = tag("path").parse(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=').parse(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_quoted_value(input)?;
    Ok((input, ComponentIdentifier::Path(value)))
}

/// Parse <Component name='value'> or <Component path='value'> opening tag
/// Returns (ComponentIdentifier, is_self_closing)
fn parse_component_opening_tag(input: &str) -> IResult<&str, (ComponentIdentifier, bool)> {
    let (input, _) = tag("<Component").parse(input)?;
    let (input, _) = space1(input)?;
    let (input, identifier) = alt((parse_name_attribute, parse_path_attribute)).parse(input)?;
    let (input, _) = space0(input)?;
    let (input, is_self_closing) =
        alt((value(true, tag("/>")), value(false, char('>')))).parse(input)?;
    Ok((input, (identifier, is_self_closing)))
}

/// Extract prefix, component content, and suffix from page template
fn extract_page_parts(input: &str) -> IResult<&str, PageParts> {
    let (input, before) = take_until("<Component").parse(input)?;
    let (input, (component_identifier, is_self_closing)) = parse_component_opening_tag(input)?;

    if is_self_closing {
        let (input, after) = rest(input)?;
        Ok((
            input,
            PageParts {
                before_component_tag: before,
                inside_component_tag: "",
                after_component_tag: after,
                component_identifier,
            },
        ))
    } else {
        let (input, inner) = take_until("</Component>").parse(input)?;
        let (input, _) = tag("</Component>").parse(input)?;
        let (input, after) = rest(input)?;
        Ok((
            input,
            PageParts {
                before_component_tag: before,
                inside_component_tag: inner,
                after_component_tag: after,
                component_identifier,
            },
        ))
    }
}

fn extract_component_start_end(input: &str) -> IResult<&str, ComponentContent> {
    // everything up to (not including) where slot_tag starts
    let (input, start) = recognize(many_till(anychar, peek(slot_tag))).parse(input)?;
    let (input, _) = slot_tag(input)?;
    let (input, end) = rest(input)?;
    Ok((
        input,
        ComponentContent {
            before_slot: start,
            after_slot: end,
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
    use serde_json::json;

    #[test]
    fn it_transforms_simple_component() {
        let data = json!(());
        let page_tmpl = "<Component name='test'>Content</Component>";
        let component_tmpl = "Header <slot /> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_component_slot_tag_with_spaces() {
        let data = json!(());
        let page_tmpl = "<Component name='test'>Content</Component>";
        let component_tmpl = "Header <  slot   /> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_component_slot_tag_with_no_spaces() {
        let data = json!(());
        let page_tmpl = "<Component name='test'>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_transforms_nested_component() {
        let data = json!(());
        let page_tmpl = "<Component name='test'><div>Content</div></Component>";
        let component_tmpl = "<header>H</header><slot /><footer>F</footer>";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "<header>H</header><div>Content</div><footer>F</footer>"
        );
    }

    #[test]
    fn it_transforms_component_with_outer_text() {
        let data = json!(());
        let page_tmpl = "before<Component name='test'><div>Content</div></Component>after";
        let component_tmpl = "<header>H</header><slot /><footer>F</footer>";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "before<header>H</header><div>Content</div><footer>F</footer>after"
        );
    }

    #[test]
    fn it_loads_component_template() {
        let data = json!(());
        let page_tmpl = "<Component name='foo'>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_parses_component_name_with_double_quotes() {
        let data = json!(());
        let page_tmpl = "<Component name=\"bar\">Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_errors_when_name_attribute_is_missing() {
        let data = json!(());
        let page_tmpl = "<Component>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_parses_component_name_with_spaces_around_equals() {
        let data = json!(());
        let page_tmpl = "<Component name = 'baz'>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_parses_component_path_attribute() {
        let data = json!(());
        let page_tmpl = "<Component path='layouts/main.html'>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_parses_component_path_with_double_quotes() {
        let data = json!(());
        let page_tmpl = "<Component path=\"layouts/main.html\">Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header Content Footer");
    }

    #[test]
    fn it_errors_when_both_name_and_path_are_present() {
        let data = json!(());
        let page_tmpl = "<Component name='foo' path='bar'>Content</Component>";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_parses_self_closing_component_tag() {
        let data = json!(());
        let page_tmpl = "<Component name='test' />";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Header  Footer");
    }

    #[test]
    fn it_parses_self_closing_component_with_path() {
        let data = json!(());
        let page_tmpl = "<Component path='layouts/main.html'/>";
        let component_tmpl = "Before <slot /> After";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "Before  After");
    }

    #[test]
    fn it_parses_self_closing_component_with_surrounding_content() {
        let data = json!(());
        let page_tmpl = "prefix<Component name='test' />suffix";
        let component_tmpl = "Header <slot/> Footer";
        let result = transform_component(page_tmpl, component_tmpl, &data);
        assert_eq!(result.unwrap(), "prefixHeader  Footersuffix");
    }
}
