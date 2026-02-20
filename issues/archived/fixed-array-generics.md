# Generalize Fixed Array Support

- [x] Improve fixed array support beyond i32 #rust-plugin 🔽 ✅ 2026-02-19

The runtime currently has only `read_fixed_i32_array<const N: usize>` and `write_fixed_i32_array<const N: usize>`. These are hardcoded to `i32`. Fixed arrays of other types (e.g., `float32[3]`, `uint8[16]`, `Point[4]`) go through the general element-by-element path.

## Current State
- `reader.rs`: `read_fixed_i32_array<const N: usize>` — reads N i32s
- `writer.rs`: `write_fixed_i32_array<const N: usize>` — writes N i32s
- Generated code for non-i32 fixed arrays uses `Default::default()` + loop fill

## Proposed Improvement
Add generic fixed array read/write for all scalar types:
```rust
pub fn read_fixed_array<T: FixedScalar, const N: usize>(&mut self) -> Result<[T; N]>
pub fn write_fixed_array<T: FixedScalar, const N: usize>(&mut self, arr: &[T; N])
```

Where `FixedScalar` is a marker trait on all fixed-size primitives.

Alternatively, keep per-type functions but add more: `read_fixed_u8_array`, `read_fixed_f32_array`, etc.

## Swift Comparison
Swift uses `InlineArray<N, Element>` with custom helpers — a richer fixed-array story.

## Note
The `Default::default()` + loop approach works correctly but requires `Default` on the element type and can't be const-evaluated. For scalar types this is fine; for compound element types it may be suboptimal.
