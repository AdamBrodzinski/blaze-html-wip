pub fn render_template_str(tmpl: &str, var_str: &str) -> String {
    tmpl.replace("@name", var_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_replaces_string_variable() {
        let tmpl = "name: @name";
        let result = render_template_str(tmpl, "Jane");
        assert_eq!(result, "name: Jane");
    }
}
