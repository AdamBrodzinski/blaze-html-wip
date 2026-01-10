use nom::Parser;
use nom::branch::alt;
use nom::multi::many0;
use nom_language::error::{VerboseError, convert_error};
use serde_json::Value;

use crate::ast::TemplateNode;

pub fn parse_template_to_ast(
    page_template: &str,
    _data: &Value,
) -> Result<Vec<TemplateNode>, String> {
    let (remaining, nodes) = many0(alt((
        crate::tag_asset::parse_script,
        crate::tag_asset::parse_style,
        crate::text::parse_text,
    )))
    .parse(page_template)
    .map_err(|e| format_verbose_error(page_template, e))?;

    if !remaining.is_empty() {
        return Err(format!(
            "Failed to parse template. Unparsed content starting at: {:?}",
            &remaining[..remaining.len().min(50)]
        ));
    }

    Ok(nodes)
}

fn format_verbose_error(input: &str, err: nom::Err<VerboseError<&str>>) -> String {
    match err {
        nom::Err::Incomplete(_) => "Incomplete input".to_string(),
        nom::Err::Error(e) | nom::Err::Failure(e) => convert_error(input, e),
    }
}

pub fn render_ast(
    ast_nodes: &Vec<TemplateNode>,
    _data: &Value,
    template_len: usize,
) -> Result<String, String> {
    let mut str_buff = String::with_capacity(template_len);
    for node in ast_nodes {
        match node {
            TemplateNode::Asset(x) => str_buff.push_str(x),
            TemplateNode::Text(x) => str_buff.push_str(x),
        }
    }
    Ok(str_buff)
}

#[cfg(test)]
mod tests {
    use super::*;

    // helper to render ast for testing
    fn render_template(page_template: &str, data: &Value) -> Result<String, String> {
        let ast_nodes = parse_template_to_ast(page_template, data).map_err(|e| e.to_string())?;
        render_ast(&ast_nodes, data, page_template.len())
    }

    mod render {
        use super::*;
        use serde_json::json;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";

        #[test]
        fn render_basic_html() {
            let template = r#"Before <Script path='test_files/asset.js' /> After"#;
            let data = json!(());
            let html = render_template(template, &data).unwrap();
            let expected =
                format!(r#"Before <script src="test_files/asset.js?{JS_HASH}"></script> After"#);
            assert_eq!(html, expected);
        }
    }

    mod parse {
        use super::*;
        use indoc::indoc;
        use serde_json::json;

        const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";
        const CSS_HASH: &str = "a0ff2dc6b477abd5ca51c463f720d3ab";

        #[test]
        fn parse_ast() {
            let template = indoc! {r#"
                Before
                <Script path="test_files/asset.js" />
                <Style path="test_files/asset.css" />
                After
            "#};
            let data = json!(());
            let ast = parse_template_to_ast(template, &data).unwrap();
            assert_eq!(ast.len(), 5);
            assert_eq!(
                ast,
                [
                    TemplateNode::Text("Before\n".into()),
                    TemplateNode::Asset(format!(
                        r#"<script src="test_files/asset.js?{JS_HASH}"></script>"#
                    )),
                    TemplateNode::Text("\n".into()),
                    TemplateNode::Asset(format!(
                        r#"<link rel="stylesheet" href="test_files/asset.css?{CSS_HASH}">"#
                    )),
                    TemplateNode::Text("\nAfter\n".into()),
                ]
            );
        }

        #[test]
        fn parse_ast_err() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js foo="bar /> After
            "#};
            let data = json!(());
            let result_err = parse_template_to_ast(template, &data).unwrap_err();
            assert!(result_err.contains("<Script"));
            assert!(result_err.contains(r#"foo="bar />"#));
        }
    }

    mod parse_err {
        use super::*;
        use indoc::indoc;
        use serde_json::json;

        #[test]
        fn missing_attr_quote() {
            let template = indoc! {r#"
                Before <Script path="test_files/asset.js" foo="bar /> After
            "#};
            let data = json!(());
            let result_err = parse_template_to_ast(template, &data).unwrap_err();
            println!("{}", &result_err);
            assert!(result_err.contains("<Script"));
            assert!(result_err.contains(r#"foo="bar />"#));
        }

        #[test]
        fn missing_asset_path_attr() {
            let template = indoc! {r#"
               <Script foo="bar" />
            "#};
            let data = json!(());
            let result_err = parse_template_to_ast(template, &data).unwrap_err();
            println!("{}", &result_err);
            assert!(result_err.contains("<Script"));
            assert!(result_err.contains("path attribute required"));
        }
    }
}
