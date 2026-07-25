use std::path::Path;

use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .register_components([("Button", Path::new("components/button.html"))])
        .unwrap()
        .build();

    blaze
        .compile_page_template("pages/bench_component_simple.html")
        .unwrap();

    let data = json!({});

    let result = blaze
        .render_page("pages/bench_component_simple.html", &data)
        .unwrap();

    println!("{}", result);
}
