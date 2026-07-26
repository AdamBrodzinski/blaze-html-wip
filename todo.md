# TODO
- X add test case for empty attr value ""
- make data Option so caller doesnt need null data

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
- extract render areas into separate files
- extract html_escape to escape module

## dig into
- slot handling/rendering
