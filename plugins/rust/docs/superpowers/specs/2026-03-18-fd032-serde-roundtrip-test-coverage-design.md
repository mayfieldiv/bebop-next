# FD-032: Serde Round-Trip Test Coverage — Design Spec

## Summary

Add 6 serde JSON round-trip tests to the integration test suite covering type categories that currently have no serde test coverage: timestamps, durations, half-precision floats, UUIDs, and maps with non-string keys.

## Context

The integration tests have serde round-trip coverage for basic types (structs, messages, enums, flags, unions) and byte arrays (added with FD-034). Several type categories with non-obvious serde behavior remain untested.

## Schema Change

Add one new type to `integration-tests/schemas/test_types.bop`:

```bop
/// Struct with a UUID field for serde testing.
struct UuidHolder {
    id: guid;
    label: string;
}
```

All other types already exist: `TimestampedEvent`, `ScheduleEntry`, `HalfPrecisionScalars`, `IntegerKeyMaps`.

## Tests

All tests go in `integration-tests/tests/integration.rs`, gated behind `#[cfg(feature = "serde")]`, following the existing pattern: construct → `serde_json::to_string` → `serde_json::from_str` → assert equal.

### 1. `serde_timestamp_roundtrip`

- **Type:** `TimestampedEvent` (struct with `BebopTimestamp` + string)
- **Setup:** `TimestampedEvent::new(BebopTimestamp { seconds: 1234567890, nanos: 500_000_000 }, "test event")`
- **Verifies:** BebopTimestamp serializes as `{"seconds":1234567890,"nanos":500000000}` object and round-trips
- **Assert:** decoded `when.seconds`, `when.nanos`, and `what` match

### 2. `serde_duration_message_roundtrip`

- **Type:** `ScheduleEntry` (message with optional `BebopTimestamp`, `BebopDuration`, string)
- **Setup:** Populate all three fields
- **Verifies:** `BebopDuration` in a message (Option-wrapped) round-trips through JSON
- **Assert:** decoded `start`, `duration`, and `label` match originals

### 2b. `serde_duration_message_partial_roundtrip`

- **Type:** `ScheduleEntry` with only `start` and `label` set (`duration` = None)
- **Verifies:** Absent `Option<BebopDuration>` serializes as null/missing and round-trips correctly
- **Assert:** decoded `duration` is `None`, other fields match

### 3. `serde_half_precision_roundtrip`

- **Type:** `HalfPrecisionScalars` (struct with `f16` + `bf16`)
- **Setup:** `HalfPrecisionScalars::new(f16::from_f32(1.5), bf16::from_f32(2.5))`
- **Verifies:** f16/bf16 survive JSON round-trip (serialized as f64 by the `half` crate)
- **Assert:** decoded values equal originals (1.5 and 2.5 are exactly representable in both f16 and bf16). Also assert intermediate JSON contains the expected numeric literals (e.g., `json.contains("1.5")`) to verify serialization format.

### 4. `serde_uuid_roundtrip`

- **Type:** `UuidHolder` (new struct with `Uuid` + string)
- **Setup:** Construct with a known UUID and label
- **Verifies:** Uuid serializes as hyphenated string and round-trips
- **Assert:** decoded `id` and `label` match

### 5. `serde_integer_key_map_roundtrip`

- **Type:** `IntegerKeyMaps` (message with `HashMap<u32, String>` and `HashMap<i64, bool>`)
- **Setup:** Populate both map fields with 2-3 entries each
- **Verifies:** Non-string map keys (u32, i64) are coerced to JSON string keys and correctly parsed back
- **Assert:** decoded maps match originals

## Files Modified

| File | Change |
|------|--------|
| `integration-tests/schemas/test_types.bop` | Add `UuidHolder` struct |
| `integration-tests/tests/integration.rs` | Add 6 serde round-trip tests |

After schema change, regenerate `integration-tests/src/test_types.rs`.

## Verification

- All 5 new tests pass with `cargo test --features serde`
- `./test.sh` passes
