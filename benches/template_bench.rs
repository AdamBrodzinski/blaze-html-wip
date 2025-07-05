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

criterion_group!(benches, bench_simple_variables, bench_deeply_nested_fields);
criterion_main!(benches);
