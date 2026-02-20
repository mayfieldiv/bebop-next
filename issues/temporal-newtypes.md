# Consider Newtype Wrappers for Timestamp/Duration

- [ ] Evaluate newtype wrappers for timestamp and duration #rust-plugin 🔽

Currently `timestamp` and `duration` map to bare `(i64, i32)` tuples (seconds + nanos). This works but provides no type safety or convenience methods.

## Current State
- Runtime: `read_timestamp() -> (i64, i32)`, `write_timestamp((i64, i32))`
- Generated code: field type is `(i64, i32)`
- Wire format: 12 bytes (i64 LE + i32 LE) per whitepaper

## Comparison
- **Production bebop**: uses `::bebop::Date` (a 64-bit tick count in .NET epoch — completely different wire format from bebop-next)
- **Swift plugin**: uses `BebopTimestamp` / `BebopDuration` newtype structs with conversion methods
- **C plugin**: uses `Bebop_Timestamp` / `Bebop_Duration` structs with `seconds`/`nanos` fields

## Proposed Improvement
Add to runtime:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BebopTimestamp { pub seconds: i64, pub nanos: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BebopDuration { pub seconds: i64, pub nanos: i32 }
```

Feature-gated conversions:
- `From<std::time::SystemTime>` / `Into<std::time::SystemTime>` for `BebopTimestamp` (under `std` feature)
- `From<std::time::Duration>` / `Into<std::time::Duration>` for `BebopDuration` (under `std` feature)
- `From<chrono::DateTime<Utc>>` for `BebopTimestamp` (under optional `chrono` feature)

## Breaking Change
This would change the generated field types from `(i64, i32)` to `BebopTimestamp`/`BebopDuration`. Since bebop-next accepts breaking changes, this is acceptable.

## Note
This is a quality-of-life improvement, not a correctness issue. The tuple representation is functionally correct.
