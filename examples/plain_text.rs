use blaze_html::BlazeTemplate;

fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_templates(["pages/bench_plain.html"])
        .unwrap();

    let result = blaze.render_page_static("pages/bench_plain.html").unwrap();

    println!("{}", result);
}
