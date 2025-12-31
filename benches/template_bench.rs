use blaze_html::each_twice::{document, process_each};
use blaze_html::BlazeTemplate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn bench_single_each_tag(c: &mut Criterion) {
    let input = b"<each-two>Inner content</each-two>";

    c.bench_function("single_each_tag", |b| b.iter(|| document(black_box(input))));
}

fn bench_sixty_each_tags(c: &mut Criterion) {
    // Build a string with 60 each tags, each on a new line
    let input: String = (0..60)
        .map(|i| format!("<each-two>Content {}</each-two>", i))
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("sixty_each_tags", |b| {
        b.iter(|| document(black_box(input.as_bytes())))
    });
}

fn bench_process_sixty_each_tags(c: &mut Criterion) {
    // Build a page template with 60 each tags
    let input: String = (0..60)
        .map(|i| format!("<Each>Content {}</Each>", i))
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("process_sixty_each_tags", |b| {
        b.iter(|| process_each(black_box(&input)))
    });
}

fn setup() -> BlazeTemplate {
    BlazeTemplate::builder()
        .set_root_directory("test_files")
        .register_component("Card", "components/card.html")
        .build()
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

fn bench_many_layout(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"name": "Jane", "age": 30});

    c.bench_function("many_layout", |b| {
        b.iter(|| blaze.render_page(black_box("bench/with_many_layout.html"), black_box(&data)))
    });
}

fn bench_many_component(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"name": "Jane", "age": 30});

    c.bench_function("many_component", |b| {
        b.iter(|| {
            blaze.render_page(
                black_box("bench/with_many_component.html"),
                black_box(&data),
            )
        })
    });
}

fn bench_each_two(c: &mut Criterion) {
    let blaze = setup();
    let data = json!({"items": [{"name": "Jane"}]});

    c.bench_function("each_two", |b| {
        b.iter(|| blaze.render_page(black_box("bench/each_two.html"), black_box(&data)))
    });
}

fn bench_many_each(c: &mut Criterion) {
    let blaze = setup();
    // Generate data with 60 arrays, each containing 1 item
    let mut data = serde_json::Map::new();
    let items: Vec<serde_json::Value> = vec![json!({"name": "Jane"})];
    for i in 0..60 {
        data.insert(
            format!("items{}", i),
            serde_json::Value::Array(items.clone()),
        );
    }
    let data = serde_json::Value::Object(data);

    c.bench_function("many_each", |b| {
        b.iter(|| blaze.render_page(black_box("bench/many_each.html"), black_box(&data)))
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
    bench_single_each_tag,
    bench_sixty_each_tags,
    bench_process_sixty_each_tags,
    bench_each_two,
    bench_many_each,
    bench_many_component,
    bench_simple_variables,
    bench_deeply_nested_fields,
    bench_layout,
    bench_many_layout,
    bench_escaped_at_symbols,
);
criterion_main!(benches);
