# Add PartialEq / Hash Derives Where Possible

- [x] Derive PartialEq, Eq, Hash on types that qualify #rust-plugin ✅

## Implemented

- **Enums / Flags**: now derive `Hash` in addition to existing `PartialEq`/`Eq`
- **Structs**:
  - always derive `PartialEq`
  - derive `Eq` when the type graph has no floating-point fields
  - derive `Hash` when the type graph has no floating-point fields and no map fields
- **Messages**:
  - always derive `PartialEq` (plus existing `Default`)
  - derive `Eq` when no floating-point fields transitively
  - derive `Hash` when no floating-point fields and no map fields transitively
- **Unions**:
  - always derive `PartialEq`
  - derive `Eq` when no floating-point branch payloads transitively
  - derive `Hash` when no floating-point or map-containing branch payloads transitively

Swift generates explicit `==` and `hash(into:)` for messages. Rust types without `PartialEq` can't be compared or used in assertions, which hurts ergonomics.

## Notes

- Float kinds considered: `f32`, `f64`, `f16`, `bf16`
- Map-containing types (`HashMap<...>`) can derive `Eq` but not `Hash`, so `Hash` is gated separately from `Eq`
- Implemented via generator pre-pass (`LifetimeAnalysis`) with transitive trait eligibility tracking

## Validation

- Added unit coverage in `plugins/rust/src/generator/mod.rs`:
  - verifies derive output for float-containing types
  - verifies map-containing types keep `Eq` but omit `Hash`
  - verifies enums include `Hash`
