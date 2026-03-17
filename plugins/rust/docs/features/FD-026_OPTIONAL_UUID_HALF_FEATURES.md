# FD-026: Optional UUID/Half Runtime Features

**Status:** Planned
**Priority:** Medium
**Effort:** Medium (2-3 hours)
**Impact:** Reduces compile time and dependency footprint for users who don't need UUID or half-precision float types

## Problem

The `bebop-runtime` crate unconditionally depends on `uuid` and `half`:

```toml
# runtime/Cargo.toml
half = { version = "2.7", default-features = false, features = ["alloc"] }
uuid = { version = "1", default-features = false }
```

Users whose schemas don't use `guid` or `float16`/`bfloat16` types still pay the compile-time cost. These aren't commonly used types — most schemas use strings, integers, and floats.

## Solution

### Runtime Changes

Make `uuid` and `half` optional dependencies, **enabled by default** so existing users aren't broken:

```toml
[features]
default = ["std", "uuid", "half"]
std = ["half?/std"]
uuid = ["dep:uuid"]
half = ["dep:half"]
```

Gate the re-exports and trait impls behind `cfg(feature = "...")`:

```rust
#[cfg(feature = "uuid")]
pub use uuid::Uuid;

#[cfg(feature = "half")]
pub use half::{bf16, f16};
```

### Generator Changes

When the generator emits code that uses `Uuid`, `f16`, or `bf16`, it should also emit a compile-time check that produces a clear error message:

```rust
// At the top of the generated file, once per needed feature
#[cfg(not(feature = "uuid"))]
compile_error!(
    "This schema uses `guid` types. Enable the `uuid` feature on bebop-runtime:\n\
     bebop-runtime = { version = \"...\", features = [\"uuid\"] }"
);
```

This way, users who disable the default features and encounter a schema with UUIDs get an actionable error instead of a confusing "cannot find type `Uuid`" message.

### Generator Detection

The generator already walks all type descriptors. Add a pass that checks if any field in the schema uses `TypeKind::Guid`, `TypeKind::Float16`, or `TypeKind::BFloat16`, and emit the corresponding `compile_error!` guard.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/Cargo.toml` | MODIFY | Make `uuid` and `half` optional, default-enabled |
| `runtime/src/lib.rs` | MODIFY | Gate re-exports behind `cfg(feature)` |
| `runtime/src/reader.rs` | MODIFY | Gate `read_guid`, `read_f16`, `read_bf16` |
| `runtime/src/writer.rs` | MODIFY | Gate `write_guid`, `write_f16`, `write_bf16` |
| `runtime/src/traits.rs` | MODIFY | Gate `FixedScalar`/`BulkScalar` impls for affected types |
| `src/generator/mod.rs` | MODIFY | Emit `compile_error!` guards based on type usage |
| `src/generator/type_mapper.rs` | MODIFY | Possibly no changes if types map unchanged |

## Verification

- `./test.sh` passes with default features (uuid + half enabled)
- Integration tests pass with `--no-default-features --features std`  on a schema that doesn't use guid/float16
- A schema using `guid` without the `uuid` feature produces the `compile_error!` message
- A schema using `float16` without the `half` feature produces the `compile_error!` message

## Design Notes

- Keeping features **enabled by default** means this is non-breaking for existing users
- The `compile_error!` approach is better than silent type absence — it's a pattern used by `prost`, `tonic`, and other Rust codegen tools
- The serde feature should conditionally activate `uuid/serde` and `half/serde` only when both the type feature and serde are enabled
