use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_variables.html")
        .unwrap();

    let data = json!({
        "first_name": "John",
        "person": {
            "last_name": "Doe",
            "age": 42_i32
        }
    });

    let result = blaze.render_page("pages/bench_variables.html", &data).unwrap();

    println!("{}", result);
}
