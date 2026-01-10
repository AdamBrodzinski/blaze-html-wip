use nom::Parser;
use nom::{branch::alt, multi::many0};
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
    .map_err(|e| e.to_string())?;

    dbg!(&nodes);
    dbg!(remaining);
    debug_assert!(remaining.is_empty());

    Ok(nodes)
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
        use indoc::indoc;
        use serde_json::json;

        #[test]
        fn parse_ast() {
            let template = indoc! {r#"
                Before
                <Script path="foo.js" />
                <Style path="bar.css" />
                After
            "#};
            let data = json!(());
            let ast = parse_template_to_ast(template, &data).unwrap();
            assert_eq!(ast.len(), 5);
            assert_eq!(
                ast,
                [
                    TemplateNode::Text("Before\n".into()),
                    TemplateNode::Asset(r#"<script src="foo.js"></script>"#.into()),
                    TemplateNode::Text("\n".into()),
                    TemplateNode::Asset(r#"<link rel="stylesheet" href="bar.css">"#.into()),
                    TemplateNode::Text("\nAfter\n".into()),
                ]
            );
        }
    }
}
