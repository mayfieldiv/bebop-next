# FD-009: Reflection Metadata

**Status:** Planned
**Priority:** Low
**Effort:** Medium (2-4 hours)
**Impact:** Enables runtime introspection, debugging, and service dispatch

## Problem

Both C and Swift generate per-type reflection metadata. The Rust plugin generates none. This is needed for runtime introspection, debugging, and service dispatch.

## Solution

Add a `BebopReflectable` trait to the runtime and generate implementations for each type:

```rust
pub trait BebopReflectable {
    fn bebop_reflection() -> &'static BebopTypeReflection;
}

pub struct BebopTypeReflection {
    pub name: &'static str,
    pub fqn: &'static str,
    pub kind: BebopTypeKind,
    pub fields: &'static [BebopFieldReflection],
}

pub struct BebopFieldReflection {
    pub name: &'static str,
    pub tag: Option<u8>,
    pub type_url: &'static str,
    pub deprecated: bool,
}
```

Consider a `no_reflection` option like C to allow opting out.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/lib.rs` | MODIFY | Add reflection types and trait |
| `src/generator/` | MODIFY | Generate `impl BebopReflectable` per type |

## Verification

- Each generated type has reflection metadata
- Reflection data matches schema definitions
- `no_reflection` option suppresses generation (if implemented)

## Related

- Related to FD-001 (services need reflection for method dispatch)
- C: `BebopReflection_DefinitionDescriptor` structs
- Swift: `BebopTypeReflection` static constants

## Source

Migrated from `../../issues/reflection.md`
