pub type TmplData = std::collections::HashMap<String, String>;

pub fn render_template_str(tmpl: &str, data: TmplData) -> String {
    let mut foo: String = String::new();
    for (key, value) in &data {
        let key_attr = format!("@{}", key);
        dbg!(&key_attr);
        dbg!(value);
        foo = tmpl.replace(&key_attr, value);
    }
    foo
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
