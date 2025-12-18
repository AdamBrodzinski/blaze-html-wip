# Layouts MVP Feature

The templates needs to handle the use case of 'layouts' in order
to wrap boilerplate html around a 'page'.

### Requirements

- layout element takes a path attribute with a filepath relative to src
- can consume global variables in layout

### Non goals

- nested layouts can be ignored
- data does not need to be passed into a layout


### Example Output

Layout template:
```html
<html>
  <head>
    <title>@title</title>
  </head>
  <body>
    <slot />
  </body>
<html>
```

Page template:
```html
<layout name="layouts/main">
  <h1>Home Page</h1>
  <p>Home info</p>
</layout>
```

Output with data {"title": "Home"}
```html
<html>
  <head>
    <title>Home</title>
  </head>
  <body>
    <h1>Home Page</h1>
    <p>Home info</p>
  </body>
<html>
```


### Usage
```rust
let layout_template = "<html><head>@title</head>@CONTENT</html>";
let page_template = "<layout><h1>Posts</h1></layout>";
let data = json!({"title": "Home"});
let result = render_template_str_with_layout(data, layout_template, page_template);
// output "<html><head>Home</head><h1>Posts</h1></html>"

pub fn handler(State(ctx): State) {
  // set in axum context
  let blaze =
    BlazeTemplate::new()
      .register_components_directory("src/components")
      .register_layouts_directory("src/layouts")
      .register_pages_directory("src/pages")
      .panic_on_error(true); // used when testing in CI

  let html =
    ctx.template_helper
     .data(json!({"title": "Home"}))
     .layout("main")
     .page("static/home/home.html")
     .render_html();

  let data = json!({"title": "Home"});

  let html = ctx.blaze
    .render_page("main_layout", "customers/view.html", data);

  let html = ctx.blaze
    .render_page("customers/view.html", data);

  return Html(html); // use axum Html response
}
```
