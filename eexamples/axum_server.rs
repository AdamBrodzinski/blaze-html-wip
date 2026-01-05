use axum::{
    extract::Path,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use blaze_html::BlazeTemplate;
use serde_json::json;
use std::sync::Arc;

struct AppState {
    blaze: BlazeTemplate,
}

#[tokio::main]
async fn main() {
    // Setup Blaze template engine
    let blaze = BlazeTemplate::builder()
        .set_root_directory("test_files")
        .register_component("Card", "components/card.html")
        .register_component("FF3Layout", "components/ff3_layout.html")
        .build();

    let state = Arc::new(AppState { blaze });

    // Build router with all benchmark example routes
    let app = Router::new()
        .route("/", get(index))
        .route("/simple-component", get(simple_component))
        .route("/simple-each", get(simple_each))
        .route("/nested-each", get(nested_each))
        .route("/ff3-homepage", get(ff3_homepage))
        .route("/ff3-videos", get(ff3_videos))
        .route("/ff3-videos/:count", get(ff3_videos_custom))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Server running on http://127.0.0.1:3000");
    println!("\nAvailable routes:");
    println!("  http://127.0.0.1:3000/                  - Index with links to all examples");
    println!("  http://127.0.0.1:3000/simple-component  - Simple component rendering (60 Cards)");
    println!("  http://127.0.0.1:3000/simple-each       - Simple Each loop rendering");
    println!("  http://127.0.0.1:3000/nested-each       - Nested Each loop rendering");
    println!("  http://127.0.0.1:3000/ff3-homepage      - FF3 homepage with poll results");
    println!("  http://127.0.0.1:3000/ff3-videos        - FF3 videos page (300 items)");
    println!("  http://127.0.0.1:3000/ff3-videos/50     - FF3 videos page (custom count)");

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> impl IntoResponse {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Blaze HTML Examples</title>
            <style>
                body {
                    font-family: system-ui, -apple-system, sans-serif;
                    max-width: 800px;
                    margin: 50px auto;
                    padding: 20px;
                    line-height: 1.6;
                }
                h1 { color: #333; }
                ul { list-style: none; padding: 0; }
                li { margin: 10px 0; }
                a {
                    color: #0066cc;
                    text-decoration: none;
                    font-size: 18px;
                }
                a:hover { text-decoration: underline; }
                .description {
                    color: #666;
                    font-size: 14px;
                    margin-left: 20px;
                }
            </style>
        </head>
        <body>
            <h1>Blaze HTML Benchmark Examples</h1>
            <p>Click on any example below to see the rendered output:</p>
            <ul>
                <li>
                    <a href="/simple-component">Simple Component</a>
                    <div class="description">Renders 60 &lt;Card&gt; components</div>
                </li>
                <li>
                    <a href="/simple-each">Simple Each Loop</a>
                    <div class="description">Renders an &lt;Each&gt; loop with 4 items</div>
                </li>
                <li>
                    <a href="/nested-each">Nested Each Loop</a>
                    <div class="description">Renders nested &lt;Each&gt; loops</div>
                </li>
                <li>
                    <a href="/ff3-homepage">FF3 Homepage</a>
                    <div class="description">FF3 homepage with layout and poll results</div>
                </li>
                <li>
                    <a href="/ff3-videos">FF3 Videos (300 items)</a>
                    <div class="description">FF3 videos page with 300 tutorial entries</div>
                </li>
                <li>
                    <a href="/ff3-videos/50">FF3 Videos (50 items)</a>
                    <div class="description">FF3 videos page with custom item count</div>
                </li>
            </ul>
        </body>
        </html>
        "#,
    )
}

// Renders 60 <Card>content</Card> components
async fn simple_component(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
    let data = json!(());

    match state
        .blaze
        .render_page("bench/component_simple.html", &data)
    {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}

// Renders 60 <Each>@name</Each> components
async fn simple_each(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
    let data = json!({
        "items": [
            {"name": "Name 1"},
            {"name": "Name 2"},
            {"name": "Name 3"},
            {"name": "Name 4"},
        ],
    });

    match state.blaze.render_page("bench/each_simple.html", &data) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}

// Renders nested <Each> with inner <Each> loop
async fn nested_each(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
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

    match state.blaze.render_page("bench/each_nested.html", &data) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}

// Renders ff3 homepage with layout component and dynamic poll results
async fn ff3_homepage(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
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

    match state
        .blaze
        .render_page("bench/ff3_homepage_template.html", &data)
    {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}

// Renders ff3 videos page with 300 tutorial entries
async fn ff3_videos(state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
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

    match state.blaze.render_page("bench/ff3_videos.html", &data) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}

// Renders ff3 videos page with custom count
async fn ff3_videos_custom(
    state: axum::extract::State<Arc<AppState>>,
    Path(count): Path<usize>,
) -> impl IntoResponse {
    let videos: Vec<_> = (0..count)
        .map(|i| {
            json!({
                "title": format!("Video {}", i + 1),
                "artist": "Anne Marie",
                "chords": "C, G, Am, Em, F",
                "link": format!("/tutorials/video-{}", i + 1)
            })
        })
        .collect();

    let data = json!({
        "videos": videos
    });

    match state.blaze.render_page("bench/ff3_videos.html", &data) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Error</h1><pre>{:?}</pre>", e)),
    }
}
