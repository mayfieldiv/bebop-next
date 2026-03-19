# FD-032: Serde Round-Trip Test Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 6 serde JSON round-trip tests covering timestamps, durations, half-precision floats, UUIDs, and integer-key maps.

**Architecture:** Add one new struct (`UuidHolder`) to the test schema, regenerate code, then add 6 test functions to `integration.rs` following the existing serde test pattern.

**Tech Stack:** Rust, serde_json, bebop schema, bebopc code generator

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `integration-tests/schemas/test_types.bop` | Modify | Add `UuidHolder` struct with `guid` + `string` fields |
| `integration-tests/src/test_types.rs` | Regenerate | Generated code — run `./generate.sh` after schema change |
| `integration-tests/tests/integration.rs` | Modify | Add 6 serde round-trip tests after existing serde tests (after line 1455) |

## Pre-Requisites

The code generator and `bebopc` binary must be available. `./generate.sh` handles building the generator and invoking bebopc. The integration test crate already has `serde_json` as a dependency and imports `Uuid`, `BebopTimestamp`, `BebopDuration`, `f16`, `bf16` from `bebop_runtime`.

---

### Task 1: Add `UuidHolder` to the test schema and regenerate

**Files:**
- Modify: `integration-tests/schemas/test_types.bop` (append before end of file, after line 269)
- Regenerate: `integration-tests/src/test_types.rs`

- [ ] **Step 1: Add `UuidHolder` struct to the schema**

Append to the end of `integration-tests/schemas/test_types.bop`:

```bop
/// Struct with a UUID field for serde round-trip testing.
struct UuidHolder {
    id: guid;
    label: string;
}
```

- [ ] **Step 2: Regenerate the test types**

Run: `./generate.sh`
Expected: "Done. Regenerated all generated files."

- [ ] **Step 3: Verify `UuidHolder` appears in generated code**

Run: `grep "pub struct UuidHolder" integration-tests/src/test_types.rs`
Expected: A struct with `pub id: ::bebop_runtime::Uuid` and `pub label: ...Cow<'buf, str>` fields.

- [ ] **Step 4: Verify the project compiles**

Run: `cargo check -p bebop-integration-tests`
Expected: success, no errors.

- [ ] **Step 5: Commit**

```bash
git add integration-tests/schemas/test_types.bop integration-tests/src/test_types.rs
git commit -m "FD-032: Add UuidHolder to test schema"
```

---

### Task 2: Add serde timestamp round-trip test

**Files:**
- Modify: `integration-tests/tests/integration.rs` (insert after `serde_union_round_trip_json` at line 1455)

- [ ] **Step 1: Write the test**

Insert after line 1455 in `integration-tests/tests/integration.rs`:

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_timestamp_roundtrip() {
  let event = TimestampedEvent::new(
    BebopTimestamp {
      seconds: 1_234_567_890,
      nanos: 500_000_000,
    },
    "test event",
  );
  let json = serde_json::to_string(&event).unwrap();
  let decoded: TimestampedEventOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.when.seconds, 1_234_567_890);
  assert_eq!(decoded.when.nanos, 500_000_000);
  assert_eq!(decoded.what, "test event");
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p bebop-integration-tests serde_timestamp_roundtrip`
Expected: 1 test passed.

---

### Task 3: Add serde duration message round-trip tests (full and partial)

**Files:**
- Modify: `integration-tests/tests/integration.rs` (append after the timestamp test)

- [ ] **Step 1: Write the full round-trip test**

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_duration_message_roundtrip() {
  let mut entry = ScheduleEntry::default();
  entry.start = Some(BebopTimestamp {
    seconds: 1_000_000,
    nanos: 0,
  });
  entry.duration = Some(BebopDuration {
    seconds: 3600,
    nanos: 500_000,
  });
  entry.label = Some(Cow::Owned("daily standup".to_string()));

  let json = serde_json::to_string(&entry).unwrap();
  let decoded: ScheduleEntryOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.start.unwrap().seconds, 1_000_000);
  assert_eq!(decoded.duration.unwrap().seconds, 3600);
  assert_eq!(decoded.duration.unwrap().nanos, 500_000);
  assert_eq!(decoded.label.as_deref(), Some("daily standup"));
}
```

- [ ] **Step 2: Write the partial round-trip test (duration absent)**

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_duration_message_partial_roundtrip() {
  let mut entry = ScheduleEntry::default();
  entry.start = Some(BebopTimestamp {
    seconds: 2_000_000,
    nanos: 100,
  });
  entry.label = Some(Cow::Owned("no duration".to_string()));
  // duration intentionally left as None

  let json = serde_json::to_string(&entry).unwrap();
  let decoded: ScheduleEntryOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.start.unwrap().seconds, 2_000_000);
  assert!(decoded.duration.is_none());
  assert_eq!(decoded.label.as_deref(), Some("no duration"));
}
```

- [ ] **Step 3: Run both tests**

Run: `cargo test -p bebop-integration-tests serde_duration_message`
Expected: 2 tests passed (the name prefix matches both).

---

### Task 4: Add serde half-precision float round-trip test

**Files:**
- Modify: `integration-tests/tests/integration.rs`

- [ ] **Step 1: Write the test**

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_half_precision_roundtrip() {
  let scalars = HalfPrecisionScalars::new(f16::from_f32(1.5), bf16::from_f32(2.5));
  let json = serde_json::to_string(&scalars).unwrap();

  // Verify serialization format contains expected numeric literals
  assert!(json.contains("1.5"), "f16 should serialize as 1.5, got: {json}");
  assert!(json.contains("2.5"), "bf16 should serialize as 2.5, got: {json}");

  let decoded: HalfPrecisionScalars = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.f16_val, f16::from_f32(1.5));
  assert_eq!(decoded.bf16_val, bf16::from_f32(2.5));
}
```

Note: 1.5 and 2.5 are exactly representable in both f16 (10-bit mantissa) and bf16 (7-bit mantissa), so the round-trip is lossless.

- [ ] **Step 2: Run the test**

Run: `cargo test -p bebop-integration-tests serde_half_precision_roundtrip`
Expected: 1 test passed.

---

### Task 5: Add serde UUID round-trip test

**Files:**
- Modify: `integration-tests/tests/integration.rs`

- [ ] **Step 1: Write the test**

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_uuid_roundtrip() {
  let id = Uuid::parse_str("e215a946-b26f-4567-a276-13136f0a1708").unwrap();
  let holder = UuidHolder::new(id, "test label");
  let json = serde_json::to_string(&holder).unwrap();

  // Verify UUID serializes as hyphenated string
  assert!(
    json.contains("e215a946-b26f-4567-a276-13136f0a1708"),
    "UUID should appear as hyphenated string, got: {json}"
  );

  let decoded: UuidHolderOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.id, id);
  assert_eq!(decoded.label, "test label");
}
```

Note: Uses the same UUID as `exampleConstGuid` in the schema for consistency. `Uuid::parse_str` is from the `uuid` crate, which is re-exported by `bebop_runtime`.

- [ ] **Step 2: Run the test**

Run: `cargo test -p bebop-integration-tests serde_uuid_roundtrip`
Expected: 1 test passed.

---

### Task 6: Add serde integer-key map round-trip test

**Files:**
- Modify: `integration-tests/tests/integration.rs`

- [ ] **Step 1: Write the test**

```rust
#[cfg(feature = "serde")]
#[test]
fn serde_integer_key_map_roundtrip() {
  let mut msg = IntegerKeyMaps::default();

  let mut labels = HashMap::new();
  labels.insert(42u32, Cow::Owned("answer".to_string()));
  labels.insert(7u32, Cow::Owned("lucky".to_string()));
  msg.labels_by_id = Some(labels);

  let mut flags = HashMap::new();
  flags.insert(-1i64, true);
  flags.insert(100i64, false);
  msg.flags_by_id = Some(flags);

  let json = serde_json::to_string(&msg).unwrap();
  let decoded: IntegerKeyMapsOwned = serde_json::from_str(&json).unwrap();

  let decoded_labels = decoded.labels_by_id.unwrap();
  assert_eq!(decoded_labels[&42], "answer");
  assert_eq!(decoded_labels[&7], "lucky");

  let decoded_flags = decoded.flags_by_id.unwrap();
  assert_eq!(decoded_flags[&-1], true);
  assert_eq!(decoded_flags[&100], false);
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p bebop-integration-tests serde_integer_key_map_roundtrip`
Expected: 1 test passed.

---

### Task 7: Final verification and commit

- [ ] **Step 1: Run all integration tests**

Run: `cargo test -p bebop-integration-tests`
Expected: All tests pass (including the 6 new serde tests).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Run the full test suite**

Run: `./test.sh`
Expected: "All checks passed."

- [ ] **Step 4: Commit all tests**

```bash
git add integration-tests/tests/integration.rs
git commit -m "FD-032: Add serde round-trip tests for timestamp, duration, f16/bf16, uuid, integer-key maps"
```
