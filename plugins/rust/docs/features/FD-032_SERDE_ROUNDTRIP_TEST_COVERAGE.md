# FD-032: Serde Round-Trip Test Coverage

**Status:** Planned
**Priority:** High
**Effort:** Low (< 1 hour)
**Impact:** Catches serde edge cases before users hit them

## Problem

The integration tests verify serde round-tripping for basic types (structs, messages, enums, unions, flags) but do NOT test several type categories that have non-obvious serde behavior:

### Untested types

1. **`BebopTimestamp` / `BebopDuration`** — Custom newtypes with manual serde impls behind `cfg(feature = "serde")`. Unknown how they serialize (struct with fields? unix timestamp number?).
2. **`f16` / `bf16`** — Half-precision floats from the `half` crate. The `half` crate serializes these as `f32` in human-readable formats (serde_json), but this may lose the "this was half-precision" semantic. Round-trip through JSON may alter precision.
3. **`Uuid`** — The `uuid` crate serializes as a hyphenated string in human-readable formats. Should work, but untested in the generated code context.
4. **`Cow<[u8]>` in non-serde_bytes context** — The generator emits `#[serde(with = "bebop_runtime::serde_cow_bytes")]` for byte array fields. But what about `Vec<Cow<[u8]>>`?  Or `HashMap<String, Cow<[u8]>>`?
5. **Maps with non-string keys** — `map[uint32, T]` generates `HashMap<u32, T>`. serde_json serializes map keys as strings, so `{42: "val"}` becomes `{"42": "val"}`. Does round-trip work?

### Risk

Users who enable serde and try to serialize these types through serde_json (the most common use case) may encounter surprising behavior or panics with no test coverage to catch it.

## Solution

Add serde JSON round-trip tests for each untested type category:

```rust
#[test]
fn serde_timestamp_roundtrip() {
    let ts = BebopTimestamp::from_secs(1234567890);
    let json = serde_json::to_string(&ts).unwrap();
    let ts2: BebopTimestamp = serde_json::from_str(&json).unwrap();
    assert_eq!(ts, ts2);
}

#[test]
fn serde_half_float_roundtrip() {
    // Verify f16 survives JSON round-trip
    let val = SomeStructWithF16 { value: f16::from_f32(1.5) };
    let json = serde_json::to_string(&val).unwrap();
    let val2: SomeStructWithF16 = serde_json::from_str(&json).unwrap();
    assert_eq!(val.value, val2.value);
}

// ... similar for uuid, byte arrays, non-string map keys
```

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `integration-tests/schemas/test_types.bop` | POSSIBLY MODIFY | May need types with timestamp/uuid/half fields if not present |
| `integration-tests/tests/integration.rs` | MODIFY | Add serde round-trip tests |
| `integration-tests/Cargo.toml` | POSSIBLY MODIFY | May need `serde_json` dev-dependency (likely already present) |

## Verification

- All new serde round-trip tests pass
- Document any surprising serialization behavior discovered (e.g., f16 precision loss)
- `./test.sh` passes
