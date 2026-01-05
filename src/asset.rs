#![allow(unused)]
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::combinator::{map, rest};
use nom::sequence::preceded;
use nom::IResult;
use serde_json::Value;

use nom::Parser;

use crate::ast::TemplateNode;
use crate::shared_parsers::parse_quoted_value;

pub fn parse_script(input: &str) -> IResult<&str, TemplateNode> {
    map(
        (
            tag("<Script"),
            space1,
            preceded(tag("path="), parse_quoted_value),
            space0,
            tag("/>"),
        ),
        |(_, _, path, _, _)| {
            let text = format!(r#"<script src="{path}"></script>"#);
            TemplateNode::Asset(text)
        },
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod script {
        use super::*;

        #[test]
        fn minimal() {
            let template = r#"<Script path="static/bar.js" />"#;
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
        }

        #[test]
        fn single_quotes() {
            let template = r#"<Script path='static/bar.js'/> other text"#;
            let (remaining, node) = parse_script(template).unwrap();
            let expected_text = r#"<script src="static/bar.js"></script>"#;
            assert_eq!(node, TemplateNode::Asset(expected_text.into()));
            assert_eq!(remaining, " other text");
        }
    }
}
