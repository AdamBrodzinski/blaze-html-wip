use blaze_html::BlazeTemplate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn setup_blaze_engine() -> BlazeTemplate {
    BlazeTemplate::builder()
        .set_root_directory("test_files")
        .register_component("Card", "components/card.html")
        .register_component("FF3Layout", "components/ff3_layout.html")
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

// renders ff3 homepage with layout component and dynamic poll results
fn bench_ff3_homepage(c: &mut Criterion) {
    let blaze = setup_blaze_engine();
    let data = json!({
        "poll_title": "November",
        "poll_results": [
            {"song": "White Christmas", "artist": "Bing Crosby", "score": 24},
            {"song": "Let It Snow", "artist": "Dean Martin", "score": 39},
            {"song": "Jingle Bells", "artist": "Frank Sinatra", "score": 15},
            {"song": "Deck the Halls", "artist": "Nat King Cole", "score": 8},
            {"song": "Happy Holiday", "artist": "Andy Williams", "score": 14},
        ],
    });

    c.bench_function("ff3_homepage", |b| {
        b.iter(|| {
            blaze.render_page(
                black_box("bench/ff3_homepage_template.html"),
                black_box(&data),
            )
        })
    });
}

// renders ff3 videos page with 300 tutorial entries
fn bench_ff3_videos(c: &mut Criterion) {
    let blaze = setup_blaze_engine();

    // Create 300 video items with same values
    let videos: Vec<_> = (0..300)
        .map(|_| {
            json!({
                "title": "2002",
                "artist": "Anne Marie",
                "chords": "C, G, Am, Em, F",
                "link": "/tutorials/2002-guitar-lesson-by-anne-marie"
            })
        })
        .collect();

    let data = json!({
        "videos": videos
    });

    c.bench_function("ff3_videos", |b| {
        b.iter(|| {
            blaze
                .render_page(black_box("bench/ff3_videos.html"), black_box(&data))
                .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_simple_component,
    bench_simple_each,
    bench_nested_each,
    bench_ff3_homepage,
    bench_ff3_videos,
);
criterion_main!(benches);
