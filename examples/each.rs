use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_templates(["pages/bench_each.html"])
        .unwrap();

    let data = json!({ "people": [{ "name": "Person 1"}, { "name": "Person 2"}] });

    let result = blaze.render_page("pages/bench_each.html", &data).unwrap();

    println!("{}", result);
}
