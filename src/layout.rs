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

/// Accept a template and only transform the <layout> section, leaving the inner contents
pub fn transform_layout(template_str: &str, data: &Value) -> Result<String, String> {
    let output = template_str.to_string();
    // let (_, parts) = parse_template(template_str).map_err(|e| e.to_string())?;
    // let mut output = String::with_capacity(template_str.len());
    //
    // for part in parts {
    //     match part {
    //         Part::Text(t) => output.push_str(t),
    //         // transform the serde Value into a String, keyed by the variable name
    //         Part::Var(var_name) => {
    //             let json_value = get_json_value_v2(data, var_name)?;
    //             match json_value {
    //                 Value::String(x) => output.push_str(x),
    //                 Value::Bool(x) => output.push_str(&x.to_string()),
    //                 Value::Number(x) => output.push_str(&x.to_string()),
    //                 Value::Null => {}
    //                 Value::Object(_) | Value::Array(_) => {}
    //             }
    //         }
    //     }
    // }
    // dbg!(&output);

    Ok(output.to_string())
}

/*

page_template_str:
<layout path="src/layouts/main.html">Middle</layout>

layout_template_str:
Before <slot/> After

TextBefore(&str)
Slot
TextAfter(&str)

"Before Middle After"

let textbefore = takeuntiltag0(|tag|  tag == "<include-script")
let slot = combo(tag("<slot"), emptyspace0, tag("/>"))
let textafter = take0()

let new_page_str = transform_layout(layout_str, page_str)

*/

// - parse page template into

// // ---------------------- variable ----------------------
//
// /// a valid variable name (after @)
// fn variable_key(input: &str) -> IResult<&str, &str> {
//     take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.').parse(input)
// }
//
// /// parse the entire variable @foo and return the variable name foo
// fn variable(input: &str) -> IResult<&str, Part> {
//     // preceeded matches the @ + var_name then discards @ tag
//     let (input, name) = preceded(tag("@"), variable_key).parse(input)?;
//     Ok((input, Part::Var(name)))
// }
//
// // ---------------------- text ----------------------
//
// fn text(input: &str) -> IResult<&str, Part> {
//     let (input, txt) = take_till1(|c| c == '@').parse(input)?;
//     Ok((input, Part::Text(txt)))
// }
//
// // ---------------------- template ----------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_replaces_variables() {
        let data = json!({"name": "Jane", "age": "45"});
        let tmpl = "name: @name end";
        let result = transform_layout(tmpl, &data);
        assert_eq!(result.unwrap(), "name: Jane end");
    }
}
