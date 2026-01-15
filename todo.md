# TODO

- X examples directory
- X benchmarks
- X cache hash on asset to_html()
- X finish caching ast on read
- X add proper errors in engine
- X escape variables
- X handle poisened lock scenario
- X handle quotes inside an attr, foo="alert('bar')"
- X preserve attr quotes, foo='bar' renders foo="bar"
- write html escape for attrs

## Future features
- LRU cache eviction

## Tests for Edge Cases
  - Empty template
  - Template with only whitespace
  - Very large templates
  - Unicode in paths/attributes
  - Path traversal attempts
