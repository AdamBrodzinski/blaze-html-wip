use nom::{IResult, bytes::complete::take_until};

use crate::ast::TemplateNode;

pub fn parse_text(input: &str) -> IResult<&str, TemplateNode> {
    println!("************************");
    let (input, text) = take_until("<Script")(input)?;
    // dbg!(input);
    // dbg!(text);
    Ok((input, TemplateNode::Text(text.to_string())))
}

/*
    match result {
        Ok((remaining, text)) if !text.is_empty() => {
            Ok((remaining, TemplateNode::Text(text.to_string())))
        }
        Ok((remaining, "")) => {
            // We're AT a <Script tag, fail so alt tries parse_script
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::TakeUntil,
            )))
        }
        Err(_) => {
            // No <Script found, consume rest (if any)
            if input.is_empty() {
                Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Eof,
                )))
            } else {
                Ok(("", TemplateNode::Text(input.to_string())))
            }
        }
    }
*/
