use nom::Parser;
use nom::{branch::alt, multi::many0};
use serde_json::Value;

use crate::asset::parse_script;
use crate::ast::TemplateNode;
use crate::text::parse_text;

/// Parse and render a template in a single pass
pub fn render_template(page_template: &str, data: &Value) -> Result<String, String> {
    let ast_nodes = parse_template_to_ast(page_template, data).map_err(|e| e.to_string())?;
    render_ast(&ast_nodes, data, page_template.len())
}

pub fn parse_template_to_ast(
    page_template: &str,
    _data: &Value,
) -> Result<Vec<TemplateNode>, String> {
    let (remaining, nodes) = many0(alt((parse_script, parse_text)))
        .parse(page_template)
        .map_err(|e| e.to_string())?;

    debug_assert!(remaining.is_empty());

    Ok(nodes)
}

pub fn render_ast(
    ast_nodes: &Vec<TemplateNode>,
    data: &Value,
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

    mod render {
        use super::*;
        use serde_json::json;

        #[test]
        fn render_basic_html() {
            let template = r#"Before <Script path='foo.js' /> After"#;
            let data = json!(());
            let html = render_template(template, &data).unwrap();
            assert_eq!(html, r#"Before <script src="foo.js"></script> After"#);
        }
    }

    mod parse {
        use super::*;
        use serde_json::json;

        #[test]
        fn parse_ast() {
            let template = r#"Before <Script path="static/bar.js" /> After"#;
            let data = json!(());
            let ast = parse_template_to_ast(template, &data).unwrap();
            assert_eq!(ast.len(), 3);
            assert_eq!(
                ast,
                [
                    TemplateNode::Text("Before ".into()),
                    TemplateNode::Asset("<script src=\"static/bar.js\"></script>".into()),
                    TemplateNode::Text(" After".into()),
                ]
            );
        }
    }
}
