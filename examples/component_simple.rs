use std::path::Path;

use blaze_html::BlazeTemplate;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .register_components([("Button", Path::new("components/button.html"))])
        .unwrap()
        .build();

    blaze
        .compile_page_templates(["pages/bench_component_simple.html"])
        .unwrap();

    let result = blaze
        .render_page_static("pages/bench_component_simple.html")
        .unwrap();

    println!("{}", result);
}
