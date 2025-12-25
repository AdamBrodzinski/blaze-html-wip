use crate::BlazeTemplate;

pub fn build_template(ctx: &BlazeTemplate, page_template: &str) -> Result<String, String> {
    dbg!(ctx);
    Ok(String::from(page_template))
}
