# FD-019: Temporal Newtypes

**Status:** Complete
**Completed:** 2026-02-20

## Summary

Added `BebopTimestamp` and `BebopDuration` newtype structs to the runtime, replacing bare `(i64, i32)` tuples. Provides type safety and convenience methods.

Wire format unchanged: 12 bytes (i64 seconds + i32 nanos). Feature-gated `From`/`Into` conversions for `std::time::SystemTime` and `std::time::Duration` under `std` feature.

## Source

Migrated from `../../issues/archived/temporal-newtypes.md`
