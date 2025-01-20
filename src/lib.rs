use regex::Regex;

pub type TmplData = std::collections::HashMap<String, String>;

pub fn render_template_str(tmpl: &str, data: TmplData) -> String {
    let reg = Regex::new(r"@(\w+)").expect("Invalid regex");

    reg.replace_all(tmpl, |caps: &regex::Captures| {
        let placeholder = caps.get(1).unwrap().as_str();
        dbg!(placeholder);
        data.get(placeholder).unwrap()
    })
    .to_string()
    //    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_replaces_string_variable() {
        let tmpl = "name: @name";
        let mut data = TmplData::new();
        data.insert("name".to_string(), "Jane".to_string());

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: Jane");
    }

    #[test]
    fn it_replaces_two_variables() {
        let tmpl = "name: @name, age: @age";
        let mut data = TmplData::new();
        data.insert("name".to_string(), "Jane".to_string());
        data.insert("age".to_string(), "45".to_string());

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: Jane, age: 45");
    }
}
