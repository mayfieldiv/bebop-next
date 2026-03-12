# FD-007: Enum Forward Compatibility

**Status:** Complete
**Completed:** 2026-03

## Summary

Implemented forward-compatible enums via the `@forward_compatible` schema decorator. Enums with this decorator get an `Unknown(T)` variant that captures unrecognized discriminator values, enabling safe round-tripping when new variants are added to schemas.

**Approach chosen:** `Unknown(T)` variant (Option B from the original design doc), not the newtype struct pattern. This preserves `match` ergonomics and mirrors how unions handle unknown discriminators.

**Two enum modes:**
- **Strict** (default): `#[repr(T)]` enum with `TryFrom` — rejects unknown values
- **Forward-compatible** (`@forward_compatible`): `Unknown(T)` variant with infallible `From<T>` — unknown values round-trip

**Generated API:**
- `discriminator(self) -> T` — raw discriminator value
- `is_known(&self) -> bool` — true if not `Unknown`
- `From<T>` (infallible) / `From<Enum> for T`

## Source

Migrated from `../../issues/plans/enum-forward-compat.md`
