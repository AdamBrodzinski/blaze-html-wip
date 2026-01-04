use blaze_html::BlazeTemplate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn setup_blaze_engine() -> BlazeTemplate {
    BlazeTemplate::builder()
        .set_root_directory("test_files")
        .register_component("Card", "components/card.html")
        .build()
}

// renders 60 <Card>content</Card> components
fn bench_simple_component(c: &mut Criterion) {
    let blaze = setup_blaze_engine();
    let data = json!(());

    c.bench_function("simple_component", |b| {
        b.iter(|| blaze.render_page(black_box("bench/component_simple.html"), black_box(&data)))
    });
}

// renders 60 <Each>@name</Each> components
fn bench_simple_each(c: &mut Criterion) {
    let blaze = setup_blaze_engine();
    let data = json!({
        "items": [
            {"name": "Name 1"},
            {"name": "Name 2"},
            {"name": "Name 3"},
            {"name": "Name 4"},
        ],
    });

    c.bench_function("simple_each", |b| {
        b.iter(|| blaze.render_page(black_box("bench/each_simple.html"), black_box(&data)))
    });
}

// renders nested <Each> with inner <Each> loop
fn bench_nested_each(c: &mut Criterion) {
    let blaze = setup_blaze_engine();
    let data = json!({
        "items": [
            {
                "item_name": "Foo",
                "item_things": [
                    {"name": "Name 1"},
                    {"name": "Name 2"},
                    {"name": "Name 3"},
                    {"name": "Name 4"},
                ],
            },
        ],
    });

    c.bench_function("nested_each", |b| {
        b.iter(|| blaze.render_page(black_box("bench/each_nested.html"), black_box(&data)))
    });
}

criterion_group!(
    benches,
    bench_simple_component,
    bench_simple_each,
    bench_nested_each,
);
criterion_main!(benches);
