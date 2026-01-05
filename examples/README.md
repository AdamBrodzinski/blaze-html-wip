# Blaze HTML Examples

This directory contains examples demonstrating how to use the Blaze HTML template engine.

## Axum Server Example

The `axum_server.rs` example demonstrates using Blaze HTML with the Axum web framework to render all the benchmark templates.

### Running the Example

```bash
cargo run --example axum_server
```

Then open your browser to http://127.0.0.1:3000

### Available Routes

- `/` - Index page with links to all examples
- `/simple-component` - Renders 60 `<Card>` components
- `/simple-each` - Simple `<Each>` loop with 4 items
- `/nested-each` - Nested `<Each>` loops
- `/ff3-homepage` - FF3 homepage with layout and poll results
- `/ff3-videos` - FF3 videos page with 300 tutorial entries
- `/ff3-videos/{count}` - FF3 videos page with custom item count

Each route uses the same templates and data as the corresponding benchmark, allowing you to see the actual rendered output before running performance tests.
