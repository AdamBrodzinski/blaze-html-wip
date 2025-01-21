use regex::Regex;

pub type TmplData = std::collections::HashMap<String, String>;

pub fn render_template_str(tmpl: &str, data: TmplData) -> String {
    let reg = Regex::new(r"@(\w+)").expect("Invalid regex");

    reg.replace_all(tmpl, |caps: &regex::Captures| {
        let key_name = caps.get(1).expect("Regex should have 1 capture").as_str();
        match data.get(key_name) {
            Some(x) => x.to_owned(),
            // fallback to no change, @foo, if hashmap.get("foo") is empty
            None => String::from(caps.get(0).unwrap().as_str()),
        }
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

    #[test]
    fn it_noops_when_data_is_not_found() {
        let tmpl = "name: @name";
        let mut data = TmplData::new();
        data.insert("never".to_string(), "matches".to_string());

        let result = render_template_str(tmpl, data);

        assert_eq!(result, "name: @name");
    }
}
