use lol_html::{doc_comments, rewrite_str, RewriteStrSettings};

pub fn rewrite_comments(template: &str) -> String {
    let settings = RewriteStrSettings {
        document_content_handlers: vec![doc_comments!(|comment| {
            comment.remove();
            Ok(())
        })],
        ..RewriteStrSettings::new()
    };
    // TODO: handle errors instead of falling back to empty template
    rewrite_str(template, settings).unwrap_or_else(|_| template.to_string())
}

#[cfg(test)]
mod tests {
    use crate::render_template_str;
    use serde_json::json;

    #[test]
    fn removes_comments_test() {
        let tmpl = "<!-- comment --><div>Hello</div>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<div>Hello</div>");
    }
}
