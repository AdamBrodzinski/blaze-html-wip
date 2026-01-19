use blaze_html::BlazeTemplate;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;

fn bench_plain_text(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();
    let data = json!({});

    blaze
        .compile_page_template("pages/bench_plain.html")
        .unwrap();

    c.bench_function("plain_text", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_plain.html"), &data))
    });
}

fn bench_asset_precompiled(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_asset.html")
        .unwrap();

    let data = json!({});

    c.bench_function("asset_precompiled", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_asset.html"), &data))
    });
}

fn bench_variables(c: &mut Criterion) {
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

    c.bench_function("variables", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_variables.html"), &data))
    });
}

fn bench_if_true(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_if_true.html")
        .unwrap();

    let data = json!({ "is_true": true });

    c.bench_function("if_true", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_if_true.html"), &data))
    });
}

fn bench_if_truthy(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_if_truthy.html")
        .unwrap();

    let data = json!({ "value": "hello world" });

    c.bench_function("if_truthy", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_if_truthy.html"), &data))
    });
}

fn bench_each(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .compile_page_template("pages/bench_each.html")
        .unwrap();

    let data = json!({ "people": [{ "name": "Person 1"}, { "name": "Person 2"}] });

    c.bench_function("each", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_each.html"), &data))
    });
}

fn bench_component(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .register_component("Person", "components/Person.html")
        .unwrap();

    blaze
        .compile_page_template("pages/bench_component.html")
        .unwrap();

    let data = json!({ "data": { "age": 30 } });

    c.bench_function("component", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_component.html"), &data))
    });
}

fn bench_component_simple(c: &mut Criterion) {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("test_files")
        .build();

    blaze
        .register_component("Button", "components/button.html")
        .unwrap();

    blaze
        .compile_page_template("pages/bench_component_simple.html")
        .unwrap();

    let data = json!({});

    c.bench_function("component simple", |b| {
        b.iter(|| blaze.render_page(black_box("pages/bench_component_simple.html"), &data))
    });
}

criterion_group!(
    benches,
    bench_component_simple,
    bench_component,
    bench_plain_text,
    bench_asset_precompiled,
    bench_variables,
    bench_if_true,
    bench_if_truthy,
    bench_each,
);
criterion_main!(benches);
