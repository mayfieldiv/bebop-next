# FD-031: Deprecated Attribute Emission

**Status:** Planned
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Compiler warnings guide users away from deprecated fields

## Problem

The Bebop schema `@deprecated` decorator is not being emitted as `#[deprecated]` on generated message fields, despite the generator having code wired for it.

Schema (test_types.bop):
```
message DeprecatedFieldsMessage {
    current_name(1): string;
    @deprecated("legacy wire compatibility")
    legacy_name(2): string;
    @deprecated
    legacy_enabled(3): bool;
}
```

Generated output (test_types.rs:2121-2125):
```rust
pub struct DeprecatedFieldsMessage<'buf> {
    pub current_name: Option<Cow<'buf, str>>,
    pub legacy_name: Option<Cow<'buf, str>>,     // missing #[deprecated]
    pub legacy_enabled: Option<bool>,              // missing #[deprecated]
}
```

Expected:
```rust
pub struct DeprecatedFieldsMessage<'buf> {
    pub current_name: Option<Cow<'buf, str>>,
    #[deprecated(note = "legacy wire compatibility")]
    pub legacy_name: Option<Cow<'buf, str>>,
    #[deprecated]
    pub legacy_enabled: Option<bool>,
}
```

## Root Cause Investigation

The generator code at `gen_message.rs:126` calls `emit_deprecated` for each field. The `emit_deprecated` function (mod.rs:711-733) checks `definition.decorators` for a `deprecated` decorator. The issue is likely one of:

1. **Field decorators not populated** — The bebopc compiler may not pass `@deprecated` on individual fields through to the `FieldDescriptor.decorators` in the `CodeGeneratorRequest`. This would be an upstream compiler issue.
2. **Decorator key mismatch** — The generator may be looking for a different decorator name than what the compiler sends.

## Investigation Steps

1. Check what `FieldDescriptor.decorators` contains for deprecated fields by examining the self-hosted descriptor types
2. Add debug logging to `emit_deprecated` for field descriptors to see if decorator data arrives
3. If the data is absent, check the bebopc compiler source to see if field-level decorators are included in the plugin protocol

## Files to Investigate/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_message.rs:126` | INVESTIGATE | Where `emit_deprecated` is called for fields |
| `src/generator/mod.rs:711-733` | INVESTIGATE | `emit_deprecated` implementation |
| `src/generated/descriptor.rs` | INVESTIGATE | Check `FieldDescriptor` for decorators field |
| bebopc compiler source | INVESTIGATE | Check if field decorators are included in plugin protocol |

## Verification

- `legacy_name` field has `#[deprecated(note = "legacy wire compatibility")]`
- `legacy_enabled` field has `#[deprecated]`
- Accessing deprecated fields produces compiler warnings
- Builder methods (FD-027) for deprecated fields also carry `#[deprecated]`
- `./test.sh` passes

## Related

- FD-002: Deprecated Field Handling (covers encode/decode behavior, not attribute emission)
- `src/generator/mod.rs:711-733` — `emit_deprecated` function
