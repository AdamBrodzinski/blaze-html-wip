use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .register_component("Person", "components/Person.html")
        .unwrap();

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
