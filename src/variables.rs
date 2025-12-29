#![allow(unused)]
use nom::bytes::complete::take_till1;
use nom::Parser;
use nom::{branch::alt, multi::many0};
use nom::{bytes::complete::tag, sequence::preceded};
use nom::{bytes::complete::take_while1, IResult};
use serde_json::Value;

use crate::data::get_json_value;

#[derive(Debug)]
enum Part<'a> {
    Text(&'a str),
    Var(&'a str),
    Escaped, // represents a literal @
}

pub fn process_variables(template_str: &str, data: &Value) -> Result<String, String> {
    let (_, parts) = parse_template(template_str).map_err(|e| e.to_string())?;
    let mut output = String::with_capacity(template_str.len());

    for part in parts {
        match part {
            Part::Text(t) => output.push_str(t),
            Part::Escaped => output.push('@'),
            // transform the serde Value into a String, keyed by the variable name
            Part::Var(var_name) => {
                let json_value = get_json_value(data, var_name)?;
                match json_value {
                    Value::String(x) => escape_html_into(x, &mut output),
                    Value::Bool(x) => output.push_str(&x.to_string()),
                    Value::Number(x) => output.push_str(&x.to_string()),
                    Value::Null => return Err(String::from("Not supported")),
                    Value::Array(arr) => {
                        return Err(format!("Cannot render array to string {arr:?}"));
                    }
                    Value::Object(obj) => {
                        return Err(format!("Cannot render obj to string {obj:?}"));
                    }
                }
            }
        }
    }

    Ok(output)
}

// ---------------------- variable ----------------------

fn variable_key(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.').parse(input)
}

// parse the entire variable @foo and return the variable name foo
fn variable(input: &str) -> IResult<&str, Part<'_>> {
    // preceeded matches the @ + var_name then discards @ tag
    let (input, name) = preceded(tag("@"), variable_key).parse(input)?;
    Ok((input, Part::Var(name)))
}

// ---------------------- escape ----------------------

fn escaped(input: &str) -> IResult<&str, Part<'_>> {
    let (input, _) = tag("@@").parse(input)?;
    Ok((input, Part::Escaped))
}

// writes directly to output buffer to intermediate allocations
fn escape_html_into(s: &str, output: &mut String) {
    if !s
        .bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\''))
    {
        output.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            _ => output.push(c),
        }
    }
}

// ---------------------- text ----------------------

fn text(input: &str) -> IResult<&str, Part<'_>> {
    let (input, txt) = take_till1(|c| c == '@').parse(input)?;
    Ok((input, Part::Text(txt)))
}

// ---------------------- template ----------------------

fn parse_template(input: &str) -> IResult<&str, Vec<Part<'_>>> {
    // note, checks escaped before variable
    many0(alt((escaped, variable, text))).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_variables() {
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name end";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_snake_case_variables() {
        let data = json!({"first_name": "Jane", "age": "45"});
        let tmpl = "name: @first_name end";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_leading_number_variables() {
        let data = json!({"2name": "Jane", "age": "45"});
        let tmpl = "name: @2name end";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }

    #[test]
    fn it_replaces_nested_variables() {
        let data = json!({"person": {"name": "Jane", "age": "45"}});
        let tmpl = "@person.name end";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Jane end");
    }

    #[test]
    fn it_renders_boolean() {
        let data = json!({"active": true});
        let tmpl = "Active: @active";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Active: true");
    }

    #[test]
    fn it_renders_integer_number() {
        let data = json!({"count": 42});
        let tmpl = "Count: @count";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Count: 42");
    }

    #[test]
    fn it_renders_float_number() {
        let data = json!({"price": 19.99});
        let tmpl = "Price: @price";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Price: 19.99");
    }

    #[test]
    fn it_errors_on_null() {
        let data = json!({"value": null});
        let tmpl = "Value: @value end";
        let result = process_variables(tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_errors_on_object() {
        let data = json!({"user": {"name": "Jane", "age": 30}});
        let tmpl = "User: @user end";
        let result = process_variables(tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_errors_on_array() {
        let data = json!({"items": [1, 2, 3]});
        let tmpl = "Items: @items end";
        let result = process_variables(tmpl, &data);
        assert!(result.is_err());
    }

    #[test]
    fn it_renders_multiple_types_in_same_template() {
        let data = json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "balance": 123.45
        });
        let tmpl = "@name is @age years old, active: @active, balance: @balance!";
        let result = process_variables(tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "Alice is 30 years old, active: true, balance: 123.45!"
        );
    }

    #[test]
    fn it_escapes_at_symbol() {
        let data = json!({});
        let tmpl = "email: user@@example.com";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "email: user@example.com");
    }

    #[test]
    fn it_escapes_css_at_rules() {
        let data = json!({});
        let tmpl = "@@media screen { }";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "@media screen { }");
    }

    #[test]
    fn it_escapes_mixed_with_variables() {
        let data = json!({"name": "Jane"});
        let tmpl = "@name's email is user@@example.com";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Jane's email is user@example.com");
    }

    #[test]
    fn it_escapes_html_in_strings() {
        let data = json!({"content": "<script>alert('xss')</script>"});
        let tmpl = "@content";
        let result = process_variables(tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn it_escapes_ampersands() {
        let data = json!({"text": "Tom & Jerry"});
        let tmpl = "@text";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "Tom &amp; Jerry");
    }

    #[test]
    fn it_escapes_quotes() {
        let data = json!({"attr": "value\" onclick=\"evil()"});
        let tmpl = "<div data-value=\"@attr\"></div>";
        let result = process_variables(tmpl, &data);
        assert_eq!(
            result.unwrap(),
            "<div data-value=\"value&quot; onclick=&quot;evil()\"></div>"
        );
    }

    #[test]
    fn it_escapes_angle_brackets() {
        let data = json!({"math": "1 < 2 > 0"});
        let tmpl = "@math";
        let result = process_variables(tmpl, &data);
        assert_eq!(result.unwrap(), "1 &lt; 2 &gt; 0");
    }
}
