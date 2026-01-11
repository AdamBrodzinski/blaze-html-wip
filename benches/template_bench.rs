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

criterion_group!(benches, bench_plain_text, bench_asset_precompiled);
criterion_main!(benches);
