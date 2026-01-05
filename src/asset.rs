use nom::bytes::complete::tag;
use nom::character::complete::space1;
use nom::IResult;
use serde_json::Value;

use nom::Parser;

pub fn parse_asset(input: &str, data: &Value) -> IResult<String, String> {
    let (input, _) = tag("<Script").parse(input)?;
    // let (input, _) = space1(input)?;

    Ok((input.to_string(), String::from("TODO")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod parse {
        use super::*;

        #[test]
        fn parse_script_tag() {
            let data = json!(());
            let template = r#"<Script src="foo.com/bar.js" />"#;
            let (input, remaining) = parse_asset(r#"<div>Foo</div>"#, &data).unwrap();
            assert_eq!(
                input,
                r#"<script src="foo.com/bar.js"></script>"#.to_string()
            );
        }
    }
}
