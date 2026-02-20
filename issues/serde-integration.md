# Feature-Gated Serde Support

- [ ] Add optional serde derives for generated types #rust-plugin 🔽

Swift generates full `Codable` conformance for all types (CodingKeys, JSON encode/decode). The Rust plugin generates no `serde` support. This is likely intentional for `no_std`, but serde is the standard for Rust serialization interop.

## Proposed Approach
Feature-gated in the runtime:
```toml
[dependencies]
serde = { version = "1", features = ["derive"], optional = true }

[features]
serde = ["dep:serde"]
```

When enabled, generated types would include:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
// or via cfg_attr:
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Foo { ... }
```

## Complications
- `Cow<'buf, str>` serializes fine with serde (as a string)
- `Cow<'buf, [u8]>` needs `#[serde(with = "serde_bytes")]` or similar for efficient byte array serialization
- `f16`/`bf16` from the `half` crate have optional serde support behind their own feature flags
- `[u8; 16]` (UUID) serializes as an array by default — may want a hex string representation
- Timestamps `(i64, i32)` serialize as tuples — may want a more structured JSON representation
- The `Unknown` variant on unions contains raw `Cow<[u8]>` which needs care

## Generator Changes
The generator would need a plugin option (e.g., `serde = true`) to conditionally emit `#[cfg_attr(feature = "serde", derive(...))]` on each type. Would also need `#[serde(rename = "originalName")]` attributes if field names differ from schema names.

## Priority
Low — this is a convenience feature. Users can manually implement serde for their types if needed.
