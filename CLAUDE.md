# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library called `blaze-html` that provides a fast, lightweight HTML template engine. It processes templates with variable substitution using the `@variable` syntax, similar to PHP or other template engines.

## Core Architecture

The library consists of a single module with two main functions:

- `render_template_str()` - Main template rendering function that processes template strings and substitutes variables
- `get_json_value()` - Helper function that extracts values from nested JSON objects using dot notation

### Template Syntax

Templates use `@variable` syntax for variable substitution:
- Simple variables: `@name`, `@age`
- Nested objects: `@person.name`, `@user.profile.email`
- Variables are only substituted when preceded by non-alphanumeric characters to avoid transforming email addresses

### Data Handling

- Templates accept `serde_json::Value` objects as data
- Supports strings, numbers, booleans, and null values
- Objects and arrays are left as-is (no substitution) until future work is completed
- Missing variables are left unchanged in the output

## Development Commands

### Build and Check
```bash
cargo build          # Build the project
cargo check          # Quick compile check
```

### Testing
```bash
cargo test           # Run all tests
cargo test --lib     # Run unit tests only
cargo test --doc     # Run documentation tests
cargo bench          # Run performance benchmarks
```

### Development
```bash
cargo doc            # Generate documentation
cargo fmt            # Format code
cargo clippy         # Run linter
```

## Project Structure

- `src/lib.rs` - Main library code with template rendering logic
- `benches/template_bench.rs` - Performance benchmarks using Criterion
- `Cargo.toml` - Project configuration with `serde_json` dependency
- `todo.md` - Template syntax examples and development notes

## Testing Strategy

The project has comprehensive unit tests covering:
- Variable substitution (simple and nested)
- Data type handling (strings, numbers, booleans, null)
- Edge cases (missing variables, email addresses)

Performance benchmarks using Criterion framework test:
- Simple variable substitution
- Boolean serialization
- Nested field access
- Deeply nested field access

## LLM Rules
- run `cargo fmt` after substantial code changes

## Adding a new template construct
- create the nom parse (for example <Foo /> tag)
- add the new parser to parse::parse_template_to_ast many0/alt
- update the text parser to stop at "<Foo"
