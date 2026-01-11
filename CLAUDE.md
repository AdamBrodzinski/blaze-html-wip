# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library called `blaze-html` that provides a fast, lightweight HTML template engine. It processes templates with variable substitution using the `@variable` syntax, similar to PHP or other template engines.

## Core Architecture

The library consists of a single module with two main public functions:

- `BlazeTemplate::new()` - template engine struct, caches ast, configures dev overrides
  - optional template engine config:
    - `.dev(true)` - disables caching for local development
    - `.template_root_dir("src")` - sets the template root dir (default: cwd)
- `.blaze_template.compile_page_template("pages/home.html", json_data)` - (optional) converts template to ast and caches for future use
- `.blaze_template.render_page("pages/home.html", json_data)` - reads template, transforms to ast, renders ast with data returning String
- `.blaze_template.render_str("tmpl_name", "Hello @name", json_data)` - reads template from str

### Template Syntax

Templates use `@variable` syntax for variable substitution:
- Simple variables: `@name`, `@age`
- Nested objects: `@person.name`, `@user.profile.email`
- Escape uses @@: `foo@@bar.com`

### Data Handling

- Templates accept `serde_json::Value` objects as data
- Supports strings, numbers, booleans, and null values
- Missing variables will return an error result

## Development Commands

### Build and Check
```bash
cargo build          # Build the project
cargo check          # Quick compile check
cargo test           # Run all tests
cargo test --lib     # Run unit tests only
cargo test --doc     # Run documentation tests
cargo bench          # Run performance benchmarks
cargo doc            # Generate documentation
cargo fmt            # Format code
cargo clippy         # Run linter
```

## Project Structure

- `src/ast.rs` - Types for AST nodes
- `src/engine.rs` - Template engine instance struct (has render_page fn)
- `src/parse.rs` - Composes all nom parsers together to parse the entire template
- `src/shared_parsers.rs` - Utility fns shared across parsers
- `src/tag_assets.rs` - Asset tags (script/style), appends cache hash to src

## Adding a new template construct
- create the nom parser (for example <Foo /> tag)
- add the new parser to parse::parse_template_to_ast many0/alt
- update the text parser to stop at "<Foo"
