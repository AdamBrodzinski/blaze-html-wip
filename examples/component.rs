use std::path::Path;

use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .register_components([("Person", Path::new("components/Person.html"))])
        .unwrap()
        .build();

    blaze
        .compile_page_template("pages/bench_component.html")
        .unwrap();

    let data = json!({
        "data": {
            "age": 30
        }
    });

    let result = blaze
        .render_page("pages/bench_component.html", &data)
        .unwrap();

    println!("{}", result);
}
