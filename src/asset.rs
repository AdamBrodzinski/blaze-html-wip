#![allow(unused)]
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::combinator::rest;
use nom::IResult;
use serde_json::Value;

use nom::Parser;

use crate::ast::TemplateNode;
use crate::shared_parsers::parse_quoted_value;

pub fn parse_script<'a>(input: &'a str, data: &Value) -> IResult<&'a str, TemplateNode> {
    let (input, _) = tag("<Script").parse(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("path=").parse(input)?;
    let (input, attr_val) = parse_quoted_value(input)?;
    dbg!(attr_val);
    let (input, _) = space0(input)?;
    let (input, _) = tag("/>").parse(input)?;
    let (_, remaining) = rest(input)?;

    let text = format!(r#"<script src="{attr_val}"></script>"#);
    Ok((remaining, TemplateNode::Asset(text)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod parse {
        use super::*;

        #[test]
        fn parse_script_tag_minimal() {
            let data = json!(());
            let template = r#"<Script path="static/bar.js" />"#;
            let (remaining, node) = parse_script(template, &data).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }

        #[test]
        fn parse_script_tag_single_quotes() {
            let data = json!(());
            let template = r#"<Script path='static/bar.js' /> other text"#;
            let (remaining, node) = parse_script(template, &data).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
            assert_eq!(remaining, " other text");
        }
    }
}
