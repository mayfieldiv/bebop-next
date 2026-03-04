# Generator-side `--serde` flag

## Problem

Generated code emits `#[cfg_attr(feature = "serde", ...)]` on all types. This requires consumers to either declare a `serde` feature in their Cargo.toml or suppress the `unexpected_cfgs` lint. Due to Rust compiler bug [#124735](https://github.com/rust-lang/rust/issues/124735), `#![allow(unexpected_cfgs)]` doesn't work at module level, making this a real friction point for consumers.

Generated code should compile cleanly in any consumer project without extra configuration.

## Design

Replace the conditional `cfg_attr` pattern with a generator-side `Serde` host option. When enabled, emit unconditional serde derives. When disabled (default), emit no serde code at all.

### GeneratorOptions

Add `serde: bool` to `GeneratorOptions`, parsed from `host_options["Serde"]` (`"true"` / `"false"`, default `false`).

### Generator output when `serde: true`

Before (conditional):
```rust
#[cfg(feature = "serde")]
use bebop_runtime::serde;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Foo { ... }
```

After (unconditional):
```rust
use bebop_runtime::serde;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Foo { ... }
```

All serde attributes become unconditional:
- `derive(serde::Serialize, serde::Deserialize)`
- `serde(borrow)`
- `serde(with = "bebop_runtime::serde_cow_bytes")`
- `serde(tag = "type", content = "value")`
- `serde(skip)`

### Generator output when `serde: false` (default)

No serde imports or attributes emitted at all.

### Files changed

1. `src/generator/mod.rs` — add `serde` to `GeneratorOptions`, conditionally emit serde import
2. `src/generator/gen_struct.rs` — conditional serde derives/attrs based on `options.serde`
3. `src/generator/gen_message.rs` — same
4. `src/generator/gen_enum.rs` — same
5. `src/generator/gen_union.rs` — same
6. `benchmarks/Cargo.toml` — remove any `[lints.rust]` workaround
7. Integration tests — generate with `Serde=true`, gate JSON round-trip tests at the test level

### Consumer experience

- Pass `Serde=true` to bebopc-gen-rust → unconditional serde, just add `serde` to dependencies
- Don't pass it → no serde code, clean compilation
- No feature flags, no lint suppression, no Cargo.toml boilerplate
