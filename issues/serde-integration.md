# Feature-Gated Serde Support

- [x] Add optional serde derives for generated types #rust-plugin 🔽

Implemented in runtime and generator on 2026-02-20.

## Implemented
Feature-gated in the runtime:
```toml
[dependencies]
serde = { version = "1", features = ["derive"], optional = true }

[features]
serde = ["dep:serde"]
```

When enabled, generated types include:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
// or via cfg_attr:
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Foo { ... }
```

## Notes
- `Cow<'buf, str>` serializes natively
- `Cow<'buf, [u8]>` uses serde bytes support
- `f16`/`bf16` serde support is enabled via the `half` crate feature
- UUID serde support is enabled via the `uuid` crate feature
- Generated code uses `cfg_attr(feature = "serde", ...)`, so serde is zero-cost when disabled

No plugin host option is required; serde is controlled by Cargo feature flags.
