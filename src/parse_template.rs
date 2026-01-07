use nom::Parser;
use nom::combinator::rest;
use nom::{branch::alt, multi::many0};
use serde_json::Value;

use crate::{asset, ast::TemplateNode};

/// Parse and render a template in a single pass
pub fn render_template(page_template: &str, data: &Value) -> Result<String, String> {
    let ast_nodes = parse_template_to_ast(page_template, data).map_err(|e| e.to_string())?;
    render_ast(ast_nodes)
}

fn parse_template_to_ast(page_template: &str, data: &Value) -> Result<Vec<TemplateNode>, String> {
    let (input, _) = many0(alt((asset::parse_script, rest)))
        .parse(page_template)
        .map_err(|e| e.to_string())?;

    Ok(Vec::new())
}

fn render_ast(ast: Vec<TemplateNode>) -> Result<String, String> {
    dbg!(ast);
    Ok(String::from("TODO"))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod script {
        use super::*;
        use serde_json::json;

        #[test]
        fn minimal() {
            let template = r#"<Script path="static/bar.js" /> After"#;
            let data = json!(());
            let ast = parse_template_to_ast(template, &data).unwrap();
            // TODO test to make sure AST matches
            assert_eq!(ast.len(), 2);
            assert_eq!(ast, []);
        }
    }
}
