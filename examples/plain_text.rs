use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_plain.html")
        .unwrap();

    let data = json!({});
    let result = blaze.render_page("pages/bench_plain.html", &data).unwrap();

    println!("{}", result);
}
