# TODO
- add register_components:
    ```
    .register_components([
        ("AppLayout", Path::new("layouts/app_layout.html")),
        ("EmptyLayout", Path::new("layouts/empty_layout.html")),
    ])?
    ```
- handle inline css styles partial
- handle cases for style tag, href-path
- ? combine escaped/raw with an escaped bool flag
- add test case for empty attr value ""
- consider raw unescaped as @@!
- make data Option so caller doesnt need null data
- X component construct
- X add <If exits="@var_name"></If>
- X rename <slot/> to <Slot/>
- X make register component return blaze instance

## Future features
- LRU cache eviction
- canonical url
- consider hoisting src/foo/bar/baz.js to assets/baz.js?1a2b3c or 1a2b3c.js
- consider style and html injection into binary
- consider style and html in render template

## Nice to have
- look into optimization for the non nested scope data case

## Tests for Edge Cases
  - Empty template
  - Template with only whitespace
  - Very large templates
  - Unicode in paths/attributes
  - Path traversal attempts


## Future refactors

- can get_include and others be consolidated into one get_file ?
- check that Include does not clash with component constant
- extract render areas into separate files
- extract html_escape to escape module

## dig into
- slot handling/rendering


