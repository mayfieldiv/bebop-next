# Optional uuid Crate Integration

- [x] Add uuid crate integration #rust-plugin ✅

Currently `uuid` maps to `[u8; 16]` — a raw byte array. This is correct and minimal, but the `uuid` crate is ubiquitous in the Rust ecosystem.

## Current State
- Runtime: `read_uuid() -> [u8; 16]`, `write_uuid(&[u8; 16])`
- Wire format: 16 bytes, canonical byte-for-byte (per whitepaper)
- Generated code: field type is `[u8; 16]`

## Comparison
- **Production bebop**: uses `::bebop::Guid` with Microsoft byte ordering (byte-swapped groups). bebop-next uses canonical ordering per the whitepaper — this is a deliberate improvement.
- **Swift plugin**: uses `Foundation.UUID`

## Proposed Approach
Feature-gated, non-breaking:
```toml
# In bebop-runtime Cargo.toml
[dependencies]
uuid = { version = "1", optional = true }

[features]
uuid = ["dep:uuid"]
```

```rust
// In runtime, under #[cfg(feature = "uuid")]
impl From<[u8; 16]> for uuid::Uuid { ... }
impl From<uuid::Uuid> for [u8; 16] { ... }
```

The generated code would continue to use `[u8; 16]` as the field type. Users opt into `uuid` interop via `.into()` conversions. No code generation changes needed.

## Alternative
Could also support generating `uuid::Uuid` as the field type when the feature is enabled, but this couples generated code to a specific optional dependency, which complicates `no_std`.
