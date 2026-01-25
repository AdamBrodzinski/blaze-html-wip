# TODO
- component construct
- combine escaped/raw with an escaped bool flag
- rename <slot/> to <Slot/>
- add <If exits="@var_name"></If>
- add case for empty attr value ""
- handle cases for style tag, href-path

## Future features
- LRU cache eviction

## Nice to have
- look into optimization for the non nested scope data case

## Tests for Edge Cases
  - Empty template
  - Template with only whitespace
  - Very large templates
  - Unicode in paths/attributes
  - Path traversal attempts
