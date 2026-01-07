use nom::Parser;
use nom::{branch::alt, multi::many0};
use serde_json::Value;

use crate::asset::parse_script;
use crate::ast::TemplateNode;
use crate::text::parse_text;

/// Parse and render a template in a single pass
pub fn render_template(page_template: &str, data: &Value) -> Result<String, String> {
    let ast_nodes = parse_template_to_ast(page_template, data).map_err(|e| e.to_string())?;
    render_ast(ast_nodes)
}

fn parse_template_to_ast(page_template: &str, data: &Value) -> Result<Vec<TemplateNode>, String> {
    let (remaining, nodes) = many0(alt((parse_script, parse_text)))
        .parse(page_template)
        .map_err(|e| e.to_string())?;

    dbg!(&remaining);
    dbg!(&nodes);

    Ok(nodes)
}

fn render_ast(ast: Vec<TemplateNode>) -> Result<String, String> {
    Ok(String::from("TODO"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // mod render {
    //     use super::*;
    //     use serde_json::json;
    //
    //     #[test]
    //     fn minimal() {
    //         let template = r#"<Script path="static/bar.js" /> After"#;
    //         let data = json!(());
    //         let html = render_template(template, &data).unwrap();
    //         assert_eq!(html, "foo");
    //     }
    // }

    mod parse {
        use super::*;
        use serde_json::json;

        #[test]
        fn minimal() {
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
