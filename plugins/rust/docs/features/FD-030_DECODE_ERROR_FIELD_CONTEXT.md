# FD-030: DecodeError Field Context

**Status:** Planned
**Priority:** Medium
**Effort:** Medium (2-3 hours)
**Impact:** Makes production debugging of wire-format issues significantly easier

## Problem

`DecodeError` reports what went wrong but not where. When decoding a complex message from network bytes, users see:

```
DecodeError: invalid utf-8 in string
```

But they don't know WHICH field in WHICH struct/message had the bad UTF-8. For a `UserProfile` with 7 fields including multiple strings, this is unhelpful.

Current error variants (from `runtime/src/error.rs`):

```rust
pub enum DecodeError {
    UnexpectedEof,
    InvalidUtf8,                              // no context at all
    InvalidEnum { type_name: &'static str, .. },  // has type name
    InvalidUnion { type_name: &'static str, .. },  // has type name
    InvalidField { type_name: &'static str, tag: u8 },  // has type + tag
    InvalidFlags { type_name: &'static str, .. },  // has type name
    AllocationFailed,
}
```

`InvalidUtf8` and `UnexpectedEof` have zero context. `InvalidEnum` has the enum type name but not which containing struct field triggered it.

## Solution

### Phase 1: Add context to contextless variants

```rust
pub enum DecodeError {
    UnexpectedEof {
        type_name: &'static str,
    },
    InvalidUtf8 {
        type_name: &'static str,
        field_name: &'static str,
    },
    // ... existing variants unchanged
}
```

### Phase 2: Thread field names through decode

In generated decode impls, wrap field decode calls to add context:

```rust
// Current:
msg.name = Some(Cow::Borrowed(reader.read_str()?));

// With context (using map_err):
msg.name = Some(Cow::Borrowed(
    reader.read_str().map_err(|e| e.with_context("UserProfile", "name"))?
));
```

Add a `with_context` method on `DecodeError` that enriches contextless variants:

```rust
impl DecodeError {
    pub fn with_context(self, type_name: &'static str, field_name: &'static str) -> Self {
        match self {
            DecodeError::UnexpectedEof => DecodeError::UnexpectedEof { type_name },
            DecodeError::InvalidUtf8 => DecodeError::InvalidUtf8 { type_name, field_name },
            other => other,  // already has context
        }
    }
}
```

### Performance Consideration

`map_err` is zero-cost on the success path — the closure only runs on error. The `&'static str` fields add no runtime allocation. The only cost is the extra `map_err` call in generated decode code, which the compiler should optimize to a branch on the error path.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/error.rs` | MODIFY | Add fields to `UnexpectedEof`/`InvalidUtf8`, add `with_context()` |
| `src/generator/gen_struct.rs` | MODIFY | Wrap field decode calls with `map_err` |
| `src/generator/gen_message.rs` | MODIFY | Wrap field decode calls with `map_err` |
| `src/generator/gen_union.rs` | MODIFY | Wrap variant decode calls with `map_err` |
| `integration-tests/tests/integration.rs` | MODIFY | Test error messages include field context |

## Verification

- Decode errors for string fields report the type and field name
- Decode errors for nested structs report the outer field context
- Existing error variant tests still pass
- Performance benchmarks show no regression on success path
- `./test.sh` passes

## Design Notes

- This is a breaking change to `DecodeError` (new fields on existing variants). Since `DecodeError` is `#[non_exhaustive]`, adding fields to struct variants is technically breaking. Consider whether to do this as new variants or modify existing ones.
- Alternative: use a wrapper `ContextualDecodeError { inner: DecodeError, path: Vec<&'static str> }` to avoid breaking the existing enum. But this adds allocation on the error path.
