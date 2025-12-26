use blaze_html::BlazeTemplate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn setup() -> BlazeTemplate {
    BlazeTemplate::new().set_root_directory("test_files")
}

fn bench_simple_variables(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"name": "Jane", "age": "45"});

    c.bench_function("simple_variables", |b| {
        b.iter(|| blaze.render_page(black_box("bench/simple_vars.html"), black_box(&data)))
    });
}

fn bench_deeply_nested_fields(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"name": "Jane", "age": 30});

    c.bench_function("deeply_nested_fields", |b| {
        b.iter(|| blaze.render_page(black_box("bench/nested_vars.html"), black_box(&data)))
    });
}

fn bench_layout(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"name": "Jane", "age": 30});

    c.bench_function("layout", |b| {
        b.iter(|| blaze.render_page(black_box("bench/with_layout.html"), black_box(&data)))
    });
}

fn bench_escaped_at_symbols(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({});

    c.bench_function("escaped_at_symbols", |b| {
        b.iter(|| blaze.render_page(black_box("bench/escaped.html"), black_box(&data)))
    });
}

// fn bench_each_tag_simple(c: &mut Criterion) {
//     let tmpl = "<each>Content to extract</each>";
//     let data = json!(());
//
//     c.bench_function("each_tag_simple", |b| {
//         b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
//     });
// }
//
// fn bench_each_tag_nested(c: &mut Criterion) {
//     let tmpl = "<each>Outer <each>Inner</each> More</each>";
//     let data = json!(());
//
//     c.bench_function("each_tag_nested", |b| {
//         b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
//     });
// }
//
// fn bench_each_tag_multiple(c: &mut Criterion) {
//     let tmpl = "<each>First</each> <each>Second</each> <each>Third</each>";
//     let data = json!(());
//
//     c.bench_function("each_tag_multiple", |b| {
//         b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
//     });
// }

criterion_group!(
    benches,
    bench_simple_variables,
    bench_deeply_nested_fields,
    bench_layout,
    bench_escaped_at_symbols
);
criterion_main!(benches);
