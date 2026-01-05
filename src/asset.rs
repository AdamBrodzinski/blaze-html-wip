use nom::bytes::complete::tag;
use nom::character::complete::space1;
use nom::IResult;
use serde_json::Value;

use nom::Parser;

pub fn parse_asset<'a>(input: &'a str, data: &Value) -> IResult<&'a str, String> {
    let (input, _) = tag("<Script").parse(input)?;
    // let (input, _) = space1(input)?;

    Ok((input, String::from("TODO")))
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
            let (remaining, output) = parse_asset(template, &data).unwrap();
            assert_eq!(
                output,
                r#"<script src="foo.com/bar.js"></script>"#.to_string()
            );
        }
    }
}
