use std::borrow::Cow;

use nom::Parser;
use nom::branch::alt;
use nom::multi::many0;
use serde_json::Value;

use crate::ast::TemplateNode;
use crate::error::BlazeError;
use crate::error::ParseErrorDetails;
use crate::template_data::get_json_value;

// Escapes HTML special characters to prevent XSS attacks.
// Returns borrowed string if no escaping needed, avoiding allocation.
fn html_escape(s: &str) -> Cow<'_, str> {
    let first_special_idx = s.find(['<', '>', '&', '"', '\'']);
    match first_special_idx {
        None => Cow::Borrowed(s),
        Some(byte_idx) => {
            let mut escaped = String::with_capacity(s.len() + 16);
            escaped.push_str(&s[..byte_idx]); // copy prefix that doesn't need escaping

            for c in s[byte_idx..].chars() {
                match c {
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '&' => escaped.push_str("&amp;"),
                    '"' => escaped.push_str("&quot;"),
                    '\'' => escaped.push_str("&#39;"),
                    _ => escaped.push(c),
                }
            }
            Cow::Owned(escaped)
        }
    }
}

pub fn parse_template_to_ast(page_template: &str) -> crate::error::Result<Vec<TemplateNode>> {
    let (remaining, nodes) = many0(alt((
        crate::tag_asset::parse_script,
        crate::tag_asset::parse_style,
        crate::variables::parse_escape,
        crate::variables::parse_variable_raw, // Must come before parse_variable (@! before @)
        crate::variables::parse_variable,
        crate::text::parse_text,
    )))
    .parse(page_template)
    .map_err(|e| BlazeError::from_nom_error(page_template, e))?;

    if !remaining.is_empty() {
        let preview: String = remaining.chars().take(50).collect();
        let position = page_template.len() - remaining.len();

        return Err(BlazeError::Parse(ParseErrorDetails::at_position(
            page_template,
            position,
            format!(
                "Failed to parse template. Unparsed content starting at: {:?}",
                preview
            ),
        )));
    }

    Ok(nodes)
}

pub fn render_ast(
    ast_nodes: &Vec<TemplateNode>,
    data: &Value,
    template_len: usize,
) -> crate::error::Result<String> {
    let mut str_buff = String::with_capacity(template_len);
    for node in ast_nodes {
        match node {
            TemplateNode::Asset(asset) => asset.write_html(&mut str_buff)?,
            TemplateNode::Escaped => str_buff.push('@'),
            TemplateNode::Text(x) => str_buff.push_str(x),
            TemplateNode::Variable(segments) => {
                let json_value = get_json_value(data, segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                match json_value {
                    Value::String(x) => str_buff.push_str(&html_escape(x)),
                    Value::Bool(x) => str_buff.push_str(&x.to_string()),
                    Value::Number(x) => str_buff.push_str(&x.to_string()),
                    Value::Null => str_buff.push_str("null"),
                    Value::Array(_) => {
                        return Err(BlazeError::render(
                            segments.join("."),
                            "cannot render array as string",
                        ));
                    }
                    Value::Object(_) => {
                        return Err(BlazeError::render(
                            segments.join("."),
                            "cannot render object as string",
                        ));
                    }
                }
            }
            TemplateNode::VariableRaw(segments) => {
                let json_value = get_json_value(data, segments)
                    .map_err(|msg| BlazeError::render(segments.join("."), msg))?;
                match json_value {
                    Value::String(x) => str_buff.push_str(x),
                    Value::Bool(x) => str_buff.push_str(&x.to_string()),
                    Value::Number(x) => str_buff.push_str(&x.to_string()),
                    Value::Null => str_buff.push_str("null"),
                    Value::Array(_) => {
                        return Err(BlazeError::render(
                            segments.join("."),
                            "cannot render array as string",
                        ));
                    }
                    Value::Object(_) => {
                        return Err(BlazeError::render(
                            segments.join("."),
                            "cannot render object as string",
                        ));
                    }
                }
            }
        }
    }
    Ok(str_buff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use serde_json::json;

    #[cfg(feature = "cache-bust")]
    mod render {
        use super::*;
        use pretty_assertions::assert_eq;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";

        // helper to render ast for testing
        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn render_basic_html() {
            let template = indoc! {r#"
                Before
                <Script path='test_files/asset.js' />
                Name: @person.first_name1
                Email: foo@@bar.com
                After
            "#};
            let data = json!({"person": {"first_name1": "Jane"}});
            let html = render_template(template, &data).unwrap();
            let expected = format!(
                indoc! {r#"
                  Before
                  <script src="test_files/asset.js?{}"></script>
                  Name: Jane
                  Email: foo@bar.com
                  After
                "#},
                JS_HASH
            );
            assert_eq!(html, expected);
        }
    }

    mod parse {
        use super::*;
        use crate::ast::{AssetKind, AssetNode};
        use pretty_assertions::assert_eq;

        #[test]
        fn parse_ast() {
            let template = indoc! {r#"
                Before
                <Script path="test_files/asset.js" />
                <Style path="test_files/asset.css" />
                foo@@bar.com
                @foo
                After
            "#};
            let ast = parse_template_to_ast(template).unwrap();
            assert_eq!(
                ast,
                [
                    TemplateNode::Text("Before\n".into()),
                    TemplateNode::Asset(AssetNode {
                        kind: AssetKind::Script,
                        path: "test_files/asset.js".to_string(),
                        attrs: vec![],
                    }),
                    TemplateNode::Text("\n".into()),
                    TemplateNode::Asset(AssetNode {
                        kind: AssetKind::Style,
                        path: "test_files/asset.css".to_string(),
                        attrs: vec![],
                    }),
                    TemplateNode::Text("\nfoo".into()),
                    TemplateNode::Escaped,
                    TemplateNode::Text("bar.com\n".into()),
                    TemplateNode::Variable(vec!["foo".into()]),
                    TemplateNode::Text("\nAfter\n".into()),
                ]
            );
        }

        #[test]
        fn parse_ast_err() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js foo="bar /> After
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("Unparsed content"));
        }
    }

    mod parse_asset_err {
        use super::*;
        use indoc::indoc;

        #[test]
        fn missing_attr_quote() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js" foo="bar /> After
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            println!("{}", &err_str);
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("missing closing quote"));
        }

        #[test]
        fn missing_asset_path_attr() {
            let template = indoc! {r#"
               Foo
               <Script foo="bar" />
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            println!("{}", &err_str);
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("path is a required attribute"));
        }

        #[test]
        fn missing_closing_tag() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js" After
            "#};
            let result_err = parse_template_to_ast(template).unwrap_err();
            let err_str = result_err.to_string();
            assert!(err_str.contains("<Script"));
            assert!(err_str.contains("Unparsed content"));
        }
    }

    mod html_escaping {
        use super::*;

        fn render_template(page_template: &str, data: &Value) -> crate::error::Result<String> {
            let ast_nodes = parse_template_to_ast(page_template)?;
            render_ast(&ast_nodes, data, page_template.len())
        }

        #[test]
        fn escapes_html_in_variables() {
            let template = "Hello @name!";
            let data = json!({"name": "<script>alert('xss')</script>"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(
                html,
                "Hello &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;!"
            );
        }

        #[test]
        fn escapes_all_special_chars() {
            let template = "@content";
            let data = json!({"content": "<>&\"'"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "&lt;&gt;&amp;&quot;&#39;");
        }

        #[test]
        fn raw_variable_does_not_escape() {
            let template = "Hello @!name!";
            let data = json!({"name": "<b>bold</b>"});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Hello <b>bold</b>!");
        }

        #[test]
        fn mixed_escaped_and_raw() {
            let template = "User: @user.name Bio: @!user.bio";
            let data = json!({"user": {"name": "<script>", "bio": "<p>Hello</p>"}});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "User: &lt;script&gt; Bio: <p>Hello</p>");
        }

        #[test]
        fn numbers_not_escaped() {
            let template = "Count: @count";
            let data = json!({"count": 42});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Count: 42");
        }

        #[test]
        fn bools_not_escaped() {
            let template = "Active: @active";
            let data = json!({"active": true});
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, "Active: true");
        }
    }
}
