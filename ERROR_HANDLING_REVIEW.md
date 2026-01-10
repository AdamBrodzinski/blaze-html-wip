# Code Review: Error Handling in blaze-html

## Overall Assessment

The error infrastructure is well-designed with `thiserror`, proper error types, and position tracking. However, there are **several issues with position accuracy** that would give users incorrect error locations.

---

## Issue 1: Wrong Position for Missing Attribute Errors (Critical)

**Location:** `src/tag_asset.rs:89-96`

```rust
None => {
    // Position is at start of input (the tag)
    let position = SourcePosition::from_offset(original_input, 0);
    return Err(BlazeError::MissingAttribute {
```

**Problem:** `original_input` is set at line 28/46 *before* any parsing occurs. The offset `0` means the error position will always be reported at the **start of the remaining input** when parsing began, not the actual tag location in the full template.

**Example:**
```html
<div>Hello</div>
<Script foo="bar" />   <!-- Missing 'path' attribute -->
```
The error will report **line 1, column 1** instead of **line 2, column 1**.

**Root Cause:** `original_input` captures where parsing *started*, not where the tag actually is in the full template. To fix this, you'd need to pass the full template and calculate the actual offset of the `<Script` tag.

---

## Issue 2: Asset Hash Errors Panic Instead of Returning Error (Critical)

**Location:** `src/tag_asset.rs:104-110`

```rust
#[cfg(feature = "cache-bust")]
fn get_cache_param(path: &str) -> String {
    let hash = hash_file(path).unwrap_or_else(|e| {
        panic!("Failed to hash asset file '{path}': {e}")
    });
    format!("?{hash}")
}
```

**Problems:**
1. **Panics crash the entire application** instead of returning `BlazeError::AssetHashFailed`
2. **No position information** - users won't know which `<Script>` or `<Style>` tag caused the error
3. The `BlazeError::AssetHashFailed` variant at `error.rs:77-84` exists but is **never used**

---

## Issue 3: nom Error Messages Use Debug Formatting

**Location:** `src/parse.rs:22`

```rust
(offset, format!("{:?}", inner.code))
```

**Problem:** This produces cryptic messages like `"TakeUntil"` or `"Tag"` instead of human-readable errors. Users see:
```
parse error at line 3, column 5: TakeUntil
```
Instead of something useful like:
```
parse error at line 3, column 5: unexpected character, expected closing tag
```

---

## Issue 4: Pointer Arithmetic Has No Safety Validation

**Location:** `src/error.rs:106-110`

```rust
pub fn calculate_offset(original: &str, remaining: &str) -> usize {
    let original_ptr = original.as_ptr() as usize;
    let remaining_ptr = remaining.as_ptr() as usize;
    remaining_ptr.saturating_sub(original_ptr)
}
```

**Concern:** This assumes `remaining` is a slice derived from `original`. If accidentally called with unrelated strings, it returns garbage (or 0 via `saturating_sub`). This could silently produce wrong positions.

Consider adding a debug assertion:
```rust
debug_assert!(remaining.as_ptr() >= original.as_ptr());
debug_assert!(remaining.as_ptr() <= unsafe { original.as_ptr().add(original.len()) });
```

---

## Issue 5: Silent Offset Clamping

**Location:** `src/error.rs:31`

```rust
let consumed = &original[..offset.min(original.len())];
```

**Minor:** If an invalid offset is passed, it's silently clamped to the string length. This prevents panics but could mask bugs upstream - the reported position would be wrong without any indication.

---

## Issue 6: Unused Error Variant

**Location:** `src/error.rs:77-84`

```rust
AssetHashFailed {
    path: PathBuf,
    message: String,
    #[source]
    source: std::io::Error,
}
```

This variant is defined but never constructed anywhere in the codebase. It was intended for the cache-bust feature but `get_cache_param` panics instead of returning this error.

---

## Summary Table

| Issue | Severity | Location | Impact |
|-------|----------|----------|--------|
| Wrong position for missing attrs | **Critical** | `tag_asset.rs:91` | User sees wrong line/column |
| Panic on asset hash failure | **Critical** | `tag_asset.rs:106-107` | Crashes instead of graceful error |
| Debug formatting in errors | Medium | `parse.rs:22` | Confusing error messages |
| Unsafe pointer arithmetic | Low | `error.rs:106-110` | Could silently produce wrong offsets |
| Silent offset clamping | Low | `error.rs:31` | Masks upstream bugs |
| Unused `AssetHashFailed` variant | Low | `error.rs:77-84` | Dead code |

---

## Recommendations

1. **For Issue 1:** Pass the full original template through the parsing pipeline and calculate the absolute offset when creating `MissingAttribute` errors

2. **For Issue 2:** Change `get_cache_param` to return `Result<String, BlazeError>` and use `BlazeError::AssetHashFailed` with position information

3. **For Issue 3:** Create human-readable error messages mapping nom error codes to descriptive strings
