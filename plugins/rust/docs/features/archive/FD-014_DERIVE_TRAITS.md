# FD-014: PartialEq / Eq / Hash Derives

**Status:** Complete
**Completed:** 2026-02-19

## Summary

Added `PartialEq`, `Eq`, and `Hash` derives on all generated types where eligible:

- **Enums/Flags:** Added `Hash`
- **Structs:** Always `PartialEq`; `Eq` when no floats; `Hash` when no floats and no maps
- **Messages:** Always `PartialEq` + `Default`; `Eq`/`Hash` gated same as structs
- **Unions:** Always `PartialEq`; `Eq`/`Hash` gated by branch payloads

Implemented via generator pre-pass (`LifetimeAnalysis`) with transitive trait eligibility tracking. Float kinds (`f32`, `f64`, `f16`, `bf16`) block `Eq`; maps additionally block `Hash`.

## Source

Migrated from `../../issues/archived/derive-traits.md`
