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

pub fn replace_variables(template_str: &str, data: &Value) -> Result<String, String> {
    let (_, parts) = parse_template(template_str).map_err(|e| e.to_string())?;
    let mut output = String::with_capacity(template_str.len());

    for part in parts {
        match part {
            Part::Text(t) => output.push_str(t),
            // transform the serde Value into a String, keyed by the variable name
            Part::Var(var_name) => {
                let json_value = get_json_value_v2(data, var_name)?;
                let value = match json_value {
                    Value::String(x) => x.to_owned(),
                    Value::Bool(x) => x.to_string(),
                    Value::Number(x) => x.to_string(),
                    Value::Null => String::new(),
                    // get_json_value will return Err if the leaf value is of type Object/Array
                    Value::Object(_) => String::from("TODO-object"),
                    // todo
                    Value::Array(_) => String::from("TODO-object"),
                };
                output.push_str(&value)
            }
        }
    }
    dbg!(&output);

    Ok(output)
}

// ---------------------- variable ----------------------

/// a valid variable name (after @)
fn variable_key(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.').parse(input)
}

/// parse the entire variable @foo and return the variable name foo
fn variable(input: &str) -> IResult<&str, Part> {
    // preceeded matches the @ + var_name then discards @ tag
    let (input, name) = preceded(tag("@"), variable_key).parse(input)?;
    Ok((input, Part::Var(name)))
}

// ---------------------- text ----------------------

fn text(input: &str) -> IResult<&str, Part> {
    let (input, txt) = take_till1(|c| c == '@').parse(input)?;
    Ok((input, Part::Text(txt)))
}

// ---------------------- template ----------------------

fn parse_template(input: &str) -> IResult<&str, Vec<Part>> {
    many0(alt((variable, text))).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_variables() {
        // Arrange
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name end";
        // Act
        let result = replace_variables(tmpl, &data);
        // Assert
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    // #[test]
    // fn it_replaces_snake_case_variables() {
    //     //let data = json!({"name": "Jane", "age": "45"});
    //     let data = String::from("Jane");
    //     let tmpl = "name: @first_name end";
    //
    //     let result = replace_variables(tmpl, data);
    //     assert_eq!(result.unwrap(), "name: Jane end");
    // }
    //
    // #[test]
    // fn it_replaces_leading_number_variables() {
    //     //let data = json!({"name": "Jane", "age": "45"});
    //     let data = String::from("Jane");
    //     let tmpl = "name: @2name end";
    //
    //     let result = replace_variables(tmpl, data);
    //     assert_eq!(result.unwrap(), "name: Jane end");
    // }
    //
    // #[test]
    // fn it_replaces_nested_variables() {
    //     //let data = json!({"name": "Jane", "age": "45"});
    //     let data = String::from("Jane");
    //     let tmpl = "@person.name end";
    //
    //     let result = replace_variables(tmpl, data);
    //     assert_eq!(result.unwrap(), "Jane end");
    // }
}
