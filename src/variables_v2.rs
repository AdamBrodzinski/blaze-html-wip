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
                match json_value {
                    Value::String(x) => output.push_str(x),
                    Value::Bool(x) => output.push_str(&x.to_string()),
                    Value::Number(x) => output.push_str(&x.to_string()),
                    Value::Null => {}
                    Value::Object(_) | Value::Array(_) => {}
                }
            }
        }
    }

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
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_snake_case_variables() {
        let data = json!({"first_name": "Jane", "age": "45"});
        let tmpl = "name: @first_name end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_leading_number_variables() {
        let data = json!({"2name": "Jane", "age": "45"});
        let tmpl = "name: @2name end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_nested_variables() {
        let data = json!({"person": {"name": "Jane", "age": "45"}});
        let tmpl = "@person.name end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Jane end");
    }

    #[test]
    fn it_renders_boolean() {
        let data = json!({"active": true});
        let tmpl = "Active: @active";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Active: true");
    }

    #[test]
    fn it_renders_integer_number() {
        let data = json!({"count": 42});
        let tmpl = "Count: @count";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Count: 42");
    }

    #[test]
    fn it_renders_float_number() {
        let data = json!({"price": 19.99});
        let tmpl = "Price: @price";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Price: 19.99");
    }

    #[test]
    fn it_renders_null_as_empty_string() {
        let data = json!({"value": null});
        let tmpl = "Value: @value end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Value:  end");
    }

    #[test]
    fn it_renders_object_as_empty_string() {
        let data = json!({"user": {"name": "Jane", "age": 30}});
        let tmpl = "User: @user end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "User:  end");
    }

    #[test]
    fn it_renders_array_as_empty_string() {
        let data = json!({"items": [1, 2, 3]});
        let tmpl = "Items: @items end";
        let result = replace_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Items:  end");
    }

    #[test]
    fn it_renders_multiple_types_in_same_template() {
        let data = json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "balance": 123.45,
            "nickname": null
        });
        let tmpl =
            "@name is @age years old, active: @active, balance: @balance, nickname: @nickname!";
        let result = replace_variables(tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "Alice is 30 years old, active: true, balance: 123.45, nickname: !"
        );
    }
}
