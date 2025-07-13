use blaze_html::render_template_str;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn bench_simple_variables(c: &mut Criterion) {
    let data = json!({"name": "Jane", "age": "45"});
    let tmpl = "name: @name, age: @age";

    c.bench_function("simple_variables", |b| {
        b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
    });
}

fn bench_deeply_nested_fields(c: &mut Criterion) {
    let tmpl = "name: @a.b.c";
    let data = json!({ "a": { "b": {"c": "foo"} } });

    c.bench_function("deeply_nested_fields", |b| {
        b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
    });
}

fn bench_each_tag_simple(c: &mut Criterion) {
    let tmpl = "<each>Content to extract</each>";
    let data = json!(());

    c.bench_function("each_tag_simple", |b| {
        b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
    });
}

fn bench_each_tag_nested(c: &mut Criterion) {
    let tmpl = "<each>Outer <each>Inner</each> More</each>";
    let data = json!(());

    c.bench_function("each_tag_nested", |b| {
        b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
    });
}

fn bench_each_tag_multiple(c: &mut Criterion) {
    let tmpl = "<each>First</each> <each>Second</each> <each>Third</each>";
    let data = json!(());

    c.bench_function("each_tag_multiple", |b| {
        b.iter(|| render_template_str(black_box(tmpl), black_box(&data)))
    });
}

criterion_group!(
    benches,
    bench_simple_variables,
    bench_deeply_nested_fields,
    bench_each_tag_simple,
    bench_each_tag_nested,
    bench_each_tag_multiple
);
criterion_main!(benches);
