# FD-010: Bulk Scalar Array Write + Read Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace per-element scalar array encode/decode with bulk memcpy on little-endian, closing 30-60,000x performance gaps vs C.

**Architecture:** Add `write_scalar_array` and `read_scalar_array` methods to the runtime that transmute `&[T]` to `&[u8]` on LE platforms. Update the code generator to emit these for all scalar array fields except `bool` (invalid bit patterns on read) and `byte` (already has its own path). No generated type changes — still `Vec<T>`, not `Cow`.

**Tech Stack:** Rust, `no_std` compatible, `unsafe` for byte transmutation with `cfg(target_endian)` gating

**Scope:** Write path + allocating read path only. Zero-copy read (`Cow<'a, [T]>`) deferred — requires lifetime analysis changes.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/writer.rs` | Modify | Add `write_scalar_array<T: FixedScalar>` |
| `runtime/src/reader.rs` | Modify | Add `read_scalar_array<T: FixedScalar>` |
| `runtime/src/traits.rs` | Modify | Add `BulkScalar` marker trait (all FixedScalar except bool) |
| `src/generator/type_mapper.rs` | Modify | Emit bulk methods for scalar array fields |
| `integration-tests/tests/integration.rs` | Modify | Add round-trip + correctness tests |

---

## Chunk 1: Runtime Methods

### Task 1: Add `BulkScalar` marker trait

The `FixedScalar` trait includes `bool`, which has invalid bit patterns (any value other than 0 or 1 is UB in Rust). We need a narrower trait for types safe to bulk-read from arbitrary wire bytes.

**Files:**
- Modify: `runtime/src/traits.rs:228-366`

- [ ] **Step 1: Add `BulkScalar` trait after `FixedScalar` definition (line 231)**

```rust
/// Marker for [`FixedScalar`] types whose every bit pattern is valid,
/// making them safe for bulk `memcpy` from wire bytes on little-endian.
///
/// `bool` is excluded: wire bytes other than 0/1 produce undefined behaviour.
///
/// # Safety
/// Implementors must have no invalid bit patterns and `size_of::<Self>()`
/// must equal the wire size of a single element.
pub unsafe trait BulkScalar: FixedScalar {}
```

- [ ] **Step 2: Implement `BulkScalar` for all `FixedScalar` types except `bool`**

```rust
unsafe impl BulkScalar for u8 {}
unsafe impl BulkScalar for i8 {}
unsafe impl BulkScalar for i16 {}
unsafe impl BulkScalar for u16 {}
unsafe impl BulkScalar for i32 {}
unsafe impl BulkScalar for u32 {}
unsafe impl BulkScalar for i64 {}
unsafe impl BulkScalar for u64 {}
unsafe impl BulkScalar for i128 {}
unsafe impl BulkScalar for u128 {}
unsafe impl BulkScalar for crate::f16 {}
unsafe impl BulkScalar for crate::bf16 {}
unsafe impl BulkScalar for f32 {}
unsafe impl BulkScalar for f64 {}
```

- [ ] **Step 3: Run tests**

Run: `cd plugins/rust && cargo test -p bebop-runtime`
Expected: PASS (no behavior change yet)

- [ ] **Step 4: Commit**

`feat: add BulkScalar marker trait for memcpy-safe scalars`

---

### Task 2: Add `write_scalar_array` to BebopWriter

**Files:**
- Modify: `runtime/src/writer.rs:173-210`

- [ ] **Step 1: Write unit test in writer.rs tests section**

There's no test module in writer.rs currently. We'll verify via integration tests. Skip to implementation.

- [ ] **Step 2: Add `write_scalar_array` after `write_array` (line 179)**

```rust
  /// Write a scalar array using bulk memcpy on little-endian.
  ///
  /// Wire format: u32 count + `count * size_of::<T>()` bytes in LE order.
  /// On big-endian, falls back to per-element writes.
  pub fn write_scalar_array<T: BulkScalar>(&mut self, items: &[T]) {
    debug_assert!(items.len() <= u32::MAX as usize, "array too large for Bebop wire format");
    self.write_u32(items.len() as u32);
    if cfg!(target_endian = "little") {
      // SAFETY: T: BulkScalar guarantees no padding and size_of::<T>() == wire size.
      // On LE, in-memory bytes ARE the wire bytes. u8 alignment is 1, always satisfied.
      let bytes = unsafe {
        core::slice::from_raw_parts(items.as_ptr() as *const u8, core::mem::size_of_val(items))
      };
      self.buf.extend_from_slice(bytes);
    } else {
      for item in items {
        item.write_to(self);
      }
    }
  }
```

Add `use crate::traits::BulkScalar;` to the imports at line 5.

- [ ] **Step 3: Also add bulk path to `write_fixed_array` (line 200)**

Replace the current per-element `write_fixed_array`:

```rust
  /// Write a fixed-size array of any scalar type (no length prefix).
  pub fn write_fixed_array<T: FixedScalar, const N: usize>(&mut self, arr: &[T; N]) {
    for item in arr {
      item.write_to(self);
    }
  }
```

With a version that detects `BulkScalar` at the call site. Actually, since we can't specialize on traits in stable Rust, leave `write_fixed_array` as-is for now — fixed arrays are small (known N at compile time) and the compiler can often unroll them. The big wins are in dynamic arrays.

- [ ] **Step 4: Run tests**

Run: `cd plugins/rust && cargo test -p bebop-runtime`
Expected: PASS

- [ ] **Step 5: Commit**

`feat: add write_scalar_array bulk memcpy path for LE`

---

### Task 3: Add `read_scalar_array` to BebopReader

**Files:**
- Modify: `runtime/src/reader.rs:199-253`

- [ ] **Step 1: Add `read_scalar_array` after `read_array` (line 221)**

```rust
  /// Read a scalar array using bulk memcpy on little-endian.
  ///
  /// Wire format: u32 count + `count * size_of::<T>()` bytes in LE order.
  /// On big-endian, falls back to per-element reads.
  pub fn read_scalar_array<T: BulkScalar>(&mut self) -> Result<Vec<T>> {
    let count = self.read_u32()? as usize;
    let elem_size = core::mem::size_of::<T>();
    let byte_len = count.checked_mul(elem_size).ok_or(DecodeError::UnexpectedEof {
      needed: usize::MAX,
      available: self.remaining(),
    })?;
    self.ensure(byte_len)?;

    if cfg!(target_endian = "little") {
      let mut vec = Vec::<T>::new();
      vec
        .try_reserve(count)
        .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
      // SAFETY: T: BulkScalar guarantees all bit patterns are valid and
      // size_of::<T>() == wire element size. On LE, wire bytes == memory layout.
      // try_reserve guarantees capacity >= count. copy_nonoverlapping is safe
      // because src (self.buf) and dst (vec) don't overlap.
      unsafe {
        core::ptr::copy_nonoverlapping(
          self.buf.as_ptr().add(self.pos),
          vec.as_mut_ptr() as *mut u8,
          byte_len,
        );
        vec.set_len(count);
      }
      self.pos += byte_len;
      Ok(vec)
    } else {
      let mut items = Vec::new();
      items
        .try_reserve(count)
        .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
      for _ in 0..count {
        items.push(T::read_from(self)?);
      }
      Ok(items)
    }
  }
```

Add `use crate::traits::BulkScalar;` to the imports at line 6.

- [ ] **Step 2: Add unit tests to existing reader test module (after line 421)**

```rust
  #[test]
  fn read_scalar_array_i32_round_trip() {
    let values: Vec<i32> = vec![1, -2, i32::MAX, i32::MIN, 0];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result: Vec<i32> = reader.read_scalar_array().unwrap();
    assert_eq!(result, values);
  }

  #[test]
  fn read_scalar_array_f64_round_trip() {
    let values: Vec<f64> = vec![1.0, -0.0, f64::INFINITY, f64::NAN, f64::MIN];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result: Vec<f64> = reader.read_scalar_array().unwrap();
    // NaN != NaN, so compare bit patterns
    for (a, b) in values.iter().zip(result.iter()) {
      assert_eq!(a.to_bits(), b.to_bits());
    }
  }

  #[test]
  fn read_scalar_array_empty() {
    let values: Vec<i64> = vec![];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result: Vec<i64> = reader.read_scalar_array().unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn read_scalar_array_malicious_count_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result: Result<Vec<i32>> = reader.read_scalar_array();
    assert!(result.is_err());
  }

  #[test]
  fn read_scalar_array_count_too_large_for_buffer() {
    // u32::MAX elements of i64 — byte_len is huge but buffer is tiny
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 64]);
    let mut reader = BebopReader::new(&buf);
    let result: Result<Vec<i64>> = reader.read_scalar_array();
    assert!(result.is_err());
  }
```

- [ ] **Step 3: Run tests**

Run: `cd plugins/rust && cargo test -p bebop-runtime`
Expected: PASS

- [ ] **Step 4: Commit**

`feat: add read_scalar_array bulk memcpy path for LE`

---

## Chunk 2: Code Generator + Integration

### Task 4: Update code generator to emit bulk methods

**Files:**
- Modify: `src/generator/type_mapper.rs:451-476` (read path), `579-613` (write path)

- [ ] **Step 1: Add `is_bulk_scalar` helper after `is_fixed_scalar` (line 105)**

```rust
/// Returns true if the TypeKind is safe for bulk memcpy (all bit patterns valid).
/// Excludes Bool (UB for non-0/1 values) and Byte (already uses
/// write_byte_array/read_byte_slice which is even better — zero-copy on read).
fn is_bulk_scalar(kind: TypeKind) -> bool {
  matches!(
    kind,
    TypeKind::Int8
      | TypeKind::Int16
      | TypeKind::Uint16
      | TypeKind::Int32
      | TypeKind::Uint32
      | TypeKind::Int64
      | TypeKind::Uint64
      | TypeKind::Int128
      | TypeKind::Uint128
      | TypeKind::Float16
      | TypeKind::Bfloat16
      | TypeKind::Float32
      | TypeKind::Float64
  )
}
```

- [ ] **Step 2: Update write path — Array case (lines 579-613)**

In the `TypeKind::Array` match arm, before the existing `scalar_write_method` check, add a bulk scalar check. The current code at line 595:

```rust
      } else if let Some(method) = scalar_write_method(elem_kind) {
```

Insert BEFORE that line:

```rust
      } else if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!(
          "{}.write_scalar_array::<{}>(&{})",
          writer, ty, value
        ));
```

- [ ] **Step 3: Update read path — Array case (lines 451-476)**

In the `TypeKind::Array` match arm, after `let elem_kind = ...` (line 464), insert a bulk scalar check BEFORE the `Defined` check:

```rust
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("{}.read_scalar_array::<{}>()", reader, ty));
      }
      if elem_kind == TypeKind::Defined {
```

- [ ] **Step 4: Run generator tests**

Run: `cd plugins/rust && cargo test`
Expected: PASS (generator unit tests verify codegen output)

- [ ] **Step 5: Commit**

`feat: emit bulk scalar array methods in code generator`

---

### Task 5: Regenerate benchmark types and verify

**Files:**
- Modify: `benchmarks/src/benchmark_types.rs` (regenerated)
- Test: `integration-tests/tests/integration.rs`

- [ ] **Step 1: Regenerate all test/benchmark schemas**

Run: `cd plugins/rust && ./test.sh`

This runs fmt, check, test, clippy — and the integration tests regenerate from `.bop` schemas.

Expected: All tests PASS. If any fail, the bulk path has a bug — investigate before proceeding.

- [ ] **Step 2: Inspect generated code to verify bulk methods are used**

Check that `benchmark_types.rs` now contains `write_scalar_array` and `read_scalar_array` calls for array fields like `item_ids`, `quantities`, `vector`, `data`:

Run: `grep -n 'scalar_array' benchmarks/src/benchmark_types.rs | head -20`

Expected: Multiple hits for `write_scalar_array::<i64>`, `read_scalar_array::<bf16>`, etc.

Also verify byte arrays still use the Cow-based zero-copy path (not the new bulk path):

Run: `grep -n 'read_byte_slice\|write_byte_array' benchmarks/src/benchmark_types.rs | head -5`

Expected: Hits confirm byte arrays didn't regress to the scalar path.

- [ ] **Step 3: Run benchmarks and compare**

Run: `cd benchmarks && cargo run --release --bin comparison`

Expected: Encode gaps for embedding/tensor/order scenarios drop from 30-500x to ~1-3x. Decode gaps should also improve significantly (bulk memcpy instead of per-element, though still allocating).

- [ ] **Step 4: Commit**

`FD-010: bulk scalar array encode/decode via memcpy on LE`

- [ ] **Step 5: Update FD-010 status**

Change status to `Pending Verification` in `docs/features/FD-010_ZERO_COPY_BULK_ARRAYS.md`.
