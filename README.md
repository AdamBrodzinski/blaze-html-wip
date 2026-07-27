# BlazeHTML

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)

<img width="1536" height="1024" alt="logo" src="https://github.com/user-attachments/assets/471588a0-4aa4-4595-b8f4-cec15d05d48a" />

Fast server-side HTML templating for Rust, designed around component-first ergonomics. Build isolated components, pass data down as props, and keep front-end-style organization without a front-end framework. BlazeHTML pairs naturally with [htmx](https://htmx.org) and [Datastar](https://data-star.dev).

- Fast iteration workflow, zero recompiles for HTML, CSS, JS changes
- Modern component organization
- Logic is written and tested in Rust, minimal markup testing
- Markup works with existing html editor tooling
- Compose generic components that pass data through as props (lexically scoped)
- Variables are automatically escaped


```html
<!-- layouts are just components with a Slot -->
<AppLayout title="Hello World">
    <!-- asset tags generate cache busting query param -->
    <Style path="assets/site.css"/>
    <Script path="assets/utils.js" defer />

    <!-- template data is passed in via JSON based ViewModel using @ variable names -->
    <h1>@page_title</h1>

    <!-- iterate over JSON arrays -->
    <Each items="@posts" as="post">
      <Card title="@post.title">
        <p>@post.desc</p>
        <If exists="@post.author">by @post.author</If>
      </Card>
    </Each>
<AppLayout>
```

```rust
let data = json!({
  page_title: "Hello World",
  posts: [
    {"title": "One", "desc": "First",  "author": "You"},
    {"title": "Two", "desc": "Second"},
  ]
});
let html = blaze.render_page("pages/home.html", &data)?;
```

Pages that take no data use `render_page_static`, which skips the empty-data argument:

```rust
let html = blaze.render_page_static("pages/about.html")?;
```

## Why

Why add another templating language? I needed a server-side templating language that did not require a Rust compilation step every time a class changed, as Askama, RSX, and similar options do. The remaining runtime-evaluated languages felt like I was developing in the 90s, Handlebars and Jinja did not support component-level organization or data decoupling via props.

BlazeHTML fills those gaps by balancing less-safe runtime evaluation with a fail-fast startup step when a template is missing. It pushes more complicated logic and testing into a Rust ViewModel struct. You can choose to build your ViewModel to fail fast if data is missing or gracefully fallback to a default/empty value. This struct can be tested in isolation and a simple Axum or Playwright smoke test can assert that the template is rendered.

BlazeHTML tries to recreate the original promise of React: a declarative function over state. The component tree receives data and declaritivly renders the view once. The client can drive further interactivity with [Datastar](https://data-star.dev), which adds fine-grained signals and declarative JavaScript interactivity while keeping as much logic on the server as possible (HTMX also provides this but it shifts more logic into the templates).


## Installation

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
blaze-html = "0.1"
serde = { version = "1", features = ["derive"] }
```

Copy the `blaze-html-skill/*` directory into your projects `.agents/skills` directory in order to speedup LLM usage. Agents typically work well without it but they tend to grep the source code and tests in order to determine what features are available.


## Quick start

Note, see Axum Quick Start for a more typical web server setup

```
project/
├── templates/
│   ├── pages/home.html
│   └── components/card.html
├── src/
│   └── main.rs
```

```html
<!-- templates/components/card.html -->
<div class="card">
  <h2>@title</h2>
  <Slot/>
</div>
```

```html
<!-- templates/pages/home.html -->
<h1>Hello @user.name</h1>

<Each items="@posts" as="post">
  <Card title="@post.title">
    <p>@post.desc</p>
  </Card>
</Each>
```

```rust
use std::path::Path;

use blaze_html::BlazeTemplate;
use serde_json::json;

fn main() -> blaze_html::Result<()> {
    // setup the template engine and register components/layouts on startup
    let blaze = BlazeTemplate::builder()
        .template_root_dir("templates")
        .register_components([("Card", Path::new("components/card.html"))])?
        .build();

    let data = json!({
        "user": { "name": "You" },
        "posts": [
            { "title": "First post", "desc": "Hello world" },
            { "title": "Second post", "desc": "Still here" },
        ]
    });

    let html = blaze.render_page("pages/home.html", &data)?;

    println!("{html}");
    Ok(())
}
```

Any `Serialize` type works as the data argument, so view models can be passed directly. Ideally all logic for transforming source data into the view model is isolated in this module boundary.

```rust
#[derive(Serialize)]
struct HomeView { user: User, posts: Vec<Post> }

let html = blaze.render_page("pages/home.html", &HomeView { user, posts })?;
```

<br/>

## Template syntax

### Variables

```html
@name                     <!-- top-level key -->
@person.name              <!-- nested object -->
@user.profile.email       <!-- arbitrarily deep -->
```

Strings, numbers, booleans, and `null` render as text. Arrays and objects are an error — render them with `<Each>` or reach a leaf value.

**Escaping.** `@variable` HTML-escapes `< > & " '`. To emit trusted markup unescaped, opt in with `@!`:

```html
@bio        <!-- escaped: &lt;b&gt;hi&lt;/b&gt; -->
@!bio       <!-- raw:     <b>hi</b> -->
```

Write a literal `@` by doubling it:

```html
foo@@bar.com     <!-- renders: foo@bar.com -->
```

A missing key is a render error, not an empty string. Use `<If exists>` when a value is genuinely optional.

### Components

Register components on the builder, then use them as tags anywhere:

```rust
use std::path::Path;

let blaze = BlazeTemplate::builder()
    .register_components([
        ("Card", Path::new("components/card.html")),
        ("Button", Path::new("components/button.html")),
    ])?
    .build();
```

String paths are also accepted, although `Path::new` is preferred.

```html
<Button label="Save"/>

<Card title="@post.title">
  <p>Children render where the component puts its <Slot/>.</p>
</Card>
```

Inside a component template, each prop is available as a top-level variable:

```html
<!-- components/card.html -->
<div class="card">
  <h2>@title</h2>
  <Slot/>
</div>
```

Props are either **static strings** (`title="Save"`) or **variable references** (`title="@post.title"`), which forward the value from the caller's scope by reference — objects and arrays pass through intact, so `<Card post="@post">` lets the component read `@post.title`.

Rules:

- Names must start with an uppercase ASCII letter, followed by letters, digits, or `_`
- `Each`, `Icon`, `If`, `Image`, `Include`, `Preload`, `Script`, `Style`, and `Slot` are reserved
- Components must be registered before the templates that use them are compiled or rendered — an unregistered tag is an error, not silent passthrough
- Children are rendered in the **caller's** scope; the component body sees its props plus the root data, but not the caller's local bindings
- Component nesting is limited to 10 levels per render; attempting an 11th level returns an error identifying the component that exceeded the limit

### Each

```html
<Each items="@people" as="person">
  <li>@person.name — @person.role</li>
</Each>
```

`items` must resolve to a JSON array. The `as` binding is scoped to the block and shadows outer names; outer data stays reachable:

```html
<Each items="@posts" as="post">
  @post.title on @site.name     <!-- @site still resolves from the root -->
</Each>
```

Nesting works as expected, with inner bindings shadowing outer ones.

### If

Five attributes, three semantics — pick the one that says what you mean:

| Attribute | Renders when |
| --- | --- |
| `true="@x"` | `x` is boolean `true` (non-booleans are an error) |
| `false="@x"` | `x` is boolean `false` |
| `truthy="@x"` | `x` is JS-like truthy (non-empty string, non-zero number, non-empty array, …) |
| `falsy="@x"` | `x` is JS-like falsy |
| `exists="@x"` | the path is present at all — never errors on a missing key |

```html
<If true="@user.is_admin">
  <a href="/admin">Admin</a>
</If>

<If exists="@flash_message">
  <div class="flash">@flash_message</div>
</If>

<If falsy="@cart.items">Your cart is empty.</If>
```

There is no `<Else>` — use a second inverted `<If>`.

### Assets

Asset tags emit the right HTML and append a content hash for cache busting:

```html
<Script path="assets/app.js" type="module"/>
<Style path="assets/site.css"/>
<Image path="assets/logo.webp" alt="Logo" width="240"/>
<Icon path="assets/favicon-32x32.png" sizes="32x32"/>
<Preload path="assets/hero.webp" as="image"/>
```

renders as:

```html
<script src="/assets/app.js?v=44f4b32954b6da985de1cfd924eacdda" type="module"></script>
<link rel="stylesheet" href="/assets/site.css?v=5c7ead8de806c5ed42f44b22c63183ee">
<img src="/assets/logo.webp?v=…" alt="Logo" width="240">
<link rel="icon" href="/assets/favicon-32x32.png?v=…" sizes="32x32">
<link rel="preload" href="/assets/hero.webp?v=…" as="image">
```

`path` is required; every other attribute passes through to the output, preserving the original quote style of valued attributes. Bare boolean attributes are accepted and normalized (`defer` renders as `defer="defer"`). The hash is the first 32 hex characters of the file's BLAKE3 digest, so the URL changes exactly when the file's bytes do.

> **Note:** asset `path` values are resolved relative to the **process working directory** (they mirror the public URL), not `template_root_dir`. A leading `/` is added if absent.

### Include

Splices a file's raw contents into the output, verbatim — no variable substitution, no tag parsing, no escaping. Useful for inlining critical CSS:

```html
<style>
  <Include path="partials/critical.css"/>
</style>
```

Include paths *are* relative to `template_root_dir`.

## Scoping

Variable resolution walks a scope chain from the innermost binding outward, falling back to the root data. Only `<Each>` bindings and component props push a scope:

```text
data:     { "site": "Blog", "posts": [ { "title": "Hi" } ] }
template: <Each items="@posts" as="post">@post.title on @site</Each>

  @post.title  ->  matches the `post` binding, then ."title"   =>  "Hi"
  @site        ->  no binding named `site`, falls to root      =>  "Blog"
```

Names and values in the chain are borrowed from your JSON and the cached AST, so rendering never deep-clones the input data.

## Configuration

```rust
let blaze = BlazeTemplate::builder()
    .template_root_dir("templates")  // default: "." (cwd)
    .dev(cfg!(debug_assertions))     // default: false
    .build();
```

| Option | Effect |
| --- | --- |
| `template_root_dir` | Root that page, component, and include paths resolve against |
| `dev` | Disables page, component, and include caching, so template edits show up on the next request |

`BlazeTemplate` is `Clone` and shares one `Arc`'d cache across clones, so cloning it into request handlers is cheap and cache hits are shared.

### Warming the cache

`compile_page_templates` parses each template, validates that every component it references is registered, and caches successful results — useful at startup so the first request doesn't pay for parsing, and so a broken template fails at boot instead of in production. Pages are processed in order; processing stops at the first error, while pages compiled before it remain cached.

```rust
blaze.compile_page_templates([
    "pages/home.html",
    "pages/about.html",
])?;
```

## Using with a web framework

`BlazeTemplate` is `Send + Sync + Clone`, so it drops straight into shared state. With [axum](https://github.com/tokio-rs/axum):

```rust
use std::path::Path;

use axum::{Router, extract::State, response::Html, routing::get};
use blaze_html::BlazeTemplate;
use serde_json::json;

#[tokio::main]
async fn main() {
    let blaze = BlazeTemplate::builder()
        .template_root_dir("templates")
        .register_components([("Card", Path::new("components/card.html"))])
        .unwrap()
        .dev(cfg!(debug_assertions))
        .build();
    blaze
        .compile_page_templates(["pages/home.html"])
        .unwrap();

    let app = Router::new()
        .route("/", get(home))
        .with_state(blaze);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn home(State(blaze): State<BlazeTemplate>) -> Html<String> {
    let data = json!({ "user": { "name": "Jane" }, "posts": [] });
    Html(blaze.render_page("pages/home.html", &data).unwrap())
}
```

## Errors

Every fallible call returns `blaze_html::Result<T>`, aliasing `Result<T, BlazeError>`:

```rust
pub enum BlazeError {
    Parse(ParseErrorDetails),      // line, column, offending line, parser context
    Io(IoErrorDetails),            // reading a template, include, or asset
    Render(RenderErrorDetails),    // missing key, wrong type, unregistered component
    Serialize(serde_json::Error),  // the data argument could not be serialized
}
```

Parse errors point at the problem:

```text
Parse error at line 12:9: path is a required attribute of <Script />

  <Script foo="bar" />
          ^

Context: asset tag -> closing tag
```

## Performance

- **Parse once.** Templates, components, and includes are parsed to an AST and cached by path; subsequent renders walk the AST directly.
- **Borrow, don't clone.** The scope chain holds `&Value` / `&str` borrows into your data and the cached AST — iterating a 10k-row `<Each>` allocates no scope names and copies no data.
- **One buffer.** Rendering writes into a single `String` pre-sized from the template length; asset tags and HTML escaping write in place, and escaping is skipped entirely for strings with nothing to escape.

Run the benchmark suite (criterion) to measure on your own hardware:

```bash
cargo bench
```

Benchmarks cover plain text, variables, `<Each>`, `<If>` (strict and truthy), asset tags, and both simple and slot-bearing components.

## Feature flags

| Flag | Default | Effect |
| --- | --- | --- |
| `cache-bust` | ✅ | Appends `?v=<blake3>` to asset URLs. Disabling it drops the `blake3` dependency and emits plain URLs. |

```toml
blaze-html = { git = "…", default-features = false }
```

## Development

```bash
cargo test          # full test suite
cargo bench         # criterion benchmarks
cargo clippy        # lints
cargo fmt           # format
cargo doc --open    # API docs
```

Runnable examples live in `examples/`:

```bash
cargo run --example variables
cargo run --example component
cargo run --example each
cargo run --example if_true
```

### Adding a template construct

1. Write the nom parser in `src/parser/` (e.g. `tag_foo.rs`)
2. Add it to the `alt` chain in `parser::parse_template_to_ast` — before `tag_component`, which matches any capitalized tag
3. Add `<Foo` to the list in `text::find_special_start` so the text parser stops there
4. Add the AST node in `src/ast.rs` and its arm in `src/render/mod.rs`

## Status

I am using this in production for project but it is still under active development. I don't expect breaking changes but there will be more features added as I run into additional use cases. See the release changelog for any breaking changes.

## License

MIT
