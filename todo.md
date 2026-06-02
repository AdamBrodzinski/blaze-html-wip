# TODO
- handle inline css styles partial
- handle cases for style tag, href-path
- ? combine escaped/raw with an escaped bool flag
- add test case for empty attr value ""
- consider raw unescaped as @@!
- X component construct
- X add <If exits="@var_name"></If>
- X rename <slot/> to <Slot/>
- X make register component return blaze instance

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


## Future refactors

- One small allocation remains that I did not touch: push_scope still does name.to_string() per scope entry (review's low-priority item). It's a tiny fixed-size string bounded by template structure, not data size — and removing it means borrowing the binding name from the AST, which re-entangles the node and data lifetimes.

## dig into
- slot handling/rendering


## bug?
// Pre-render the slot content (the component's children) against the parent
// context. Slot content is evaluated in the caller's scope, which does not
// change while the component body renders, so rendering it once up front is
// equivalent to rendering it lazily at each <Slot/> — and needs no aliasing
// of the parent and component contexts. Skipped when the template has no slot
// so unused children keep their original "not evaluated" behavior.
