# FD-023: Zero-Copy Scalar Array Decode — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `read_scalar_array` to return `Cow<'a, [T]>` (zero-copy borrow on aligned LE, owned fallback otherwise), and update the generator to emit `Cow<'buf, [T]>` field types for all bulk scalar arrays.

**Architecture:** Runtime method returns `Cow` with alignment-gated borrowing. Generator's lifetime analysis, type mapping, and Cow helpers are extended to treat bulk scalar arrays like byte arrays. Generated struct/message types gain lifetimes where needed.

**Tech Stack:** Rust, bebop-runtime, bebopc-gen-rust code generator

**Spec:** `docs/superpowers/specs/2026-03-17-fd023-zero-copy-scalar-array-decode-design.md`

---

## Task 1: Runtime — Replace `read_scalar_array` to return `Cow`

**Files:**
- Modify: `runtime/src/reader.rs:1-11` (imports), `runtime/src/reader.rs:224-268` (method body)

- [ ] **Step 1: Add `Cow` import to reader.rs**

At line 1, add `use alloc::borrow::Cow;` to the existing imports.

```rust
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
```

- [ ] **Step 2: Replace `read_scalar_array` implementation**

Replace lines 224-268 (`read_scalar_array` method) with the zero-copy version. Same name, return type changes from `Result<Vec<T>>` to `Result<Cow<'a, [T]>>`:

```rust
  /// Read a scalar array, borrowing directly from the buffer when possible.
  ///
  /// Wire format: u32 count + `count * size_of::<T>()` bytes in LE order.
  /// On little-endian with aligned data, returns `Cow::Borrowed` (zero-copy).
  /// On little-endian with unaligned data, returns `Cow::Owned` (bulk memcpy).
  /// On big-endian, falls back to per-element reads into `Cow::Owned`.
  pub fn read_scalar_array<T: BulkScalar>(&mut self) -> Result<Cow<'a, [T]>> {
    let count = self.read_u32()? as usize;
    if count == 0 {
      return Ok(Cow::Borrowed(&[]));
    }
    let elem_size = core::mem::size_of::<T>();
    let byte_len = count
      .checked_mul(elem_size)
      .ok_or(DecodeError::UnexpectedEof {
        needed: usize::MAX,
        available: self.remaining(),
      })?;
    self.ensure(byte_len)?;

    if cfg!(target_endian = "little") {
      let ptr = self.buf.as_ptr().wrapping_add(self.pos);
      if ptr.align_offset(core::mem::align_of::<T>()) == 0 {
        // SAFETY: T: BulkScalar guarantees all bit patterns are valid and
        // size_of::<T>() == wire element size. On LE, wire bytes == memory
        // layout. align_offset confirmed pointer alignment. ensure() confirmed
        // byte_len bytes are available. The resulting slice borrows from
        // self.buf with lifetime 'a.
        let slice = unsafe {
          core::slice::from_raw_parts(ptr as *const T, count)
        };
        self.pos += byte_len;
        Ok(Cow::Borrowed(slice))
      } else {
        // Unaligned: bulk memcpy into Vec
        let mut vec = Vec::<T>::new();
        vec
          .try_reserve(count)
          .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
        // SAFETY: Same as above minus alignment — we copy into a properly
        // aligned Vec instead. try_reserve guarantees capacity >= count.
        // copy_nonoverlapping is safe because src (self.buf) and dst (vec)
        // don't overlap.
        unsafe {
          core::ptr::copy_nonoverlapping(
            self.buf.as_ptr().add(self.pos),
            vec.as_mut_ptr() as *mut u8,
            byte_len,
          );
          vec.set_len(count);
        }
        self.pos += byte_len;
        Ok(Cow::Owned(vec))
      }
    } else {
      let mut items = Vec::new();
      items
        .try_reserve(count)
        .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
      for _ in 0..count {
        items.push(T::read_from(self)?);
      }
      Ok(Cow::Owned(items))
    }
  }
```

- [ ] **Step 3: Update existing tests for new return type**

The existing tests in `reader.rs:502-556` reference `Vec<T>` in type annotations and assertions. Update them:

**`read_scalar_array_i32_round_trip`** (line 503-511): Change `result` usage — `Cow` derefs to `&[T]`, so compare with `assert_eq!(*result, *values)` or convert:

```rust
  #[test]
  fn read_scalar_array_i32_round_trip() {
    let values: Vec<i32> = vec![1, -2, i32::MAX, i32::MIN, 0];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
  }
```

**`read_scalar_array_f64_round_trip`** (line 513-525): Same pattern, compare bit patterns via `&*result`:

```rust
  #[test]
  fn read_scalar_array_f64_round_trip() {
    let values: Vec<f64> = vec![1.0, -0.0, f64::INFINITY, f64::NAN, f64::MIN];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<f64>().unwrap();
    for (a, b) in values.iter().zip(result.iter()) {
      assert_eq!(a.to_bits(), b.to_bits());
    }
  }
```

**`read_scalar_array_empty`** (line 527-536): Verify it returns `Cow::Borrowed`:

```rust
  #[test]
  fn read_scalar_array_empty() {
    let values: Vec<i64> = vec![];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i64>().unwrap();
    assert!(result.is_empty());
    assert!(matches!(result, Cow::Borrowed(_)));
  }
```

**`read_scalar_array_malicious_count_returns_error`** (line 538-546): Remove explicit `Vec<i32>` type annotation — infer `Cow`:

```rust
  #[test]
  fn read_scalar_array_malicious_count_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_scalar_array::<i32>();
    assert!(result.is_err());
  }
```

**`read_scalar_array_count_too_large_for_buffer`** (line 548-556): Same fix:

```rust
  #[test]
  fn read_scalar_array_count_too_large_for_buffer() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 64]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_scalar_array::<i64>();
    assert!(result.is_err());
  }
```

- [ ] **Step 4: Add new zero-copy-specific tests**

Add after the existing tests:

```rust
  #[test]
  fn read_scalar_array_aligned_returns_borrowed() {
    // BebopWriter produces: 4-byte count + N*4 bytes of i32 data.
    // The buffer starts at a Vec allocation (aligned), so data at offset 4
    // is aligned for i32 (align=4) when the Vec base is aligned.
    let values: Vec<i32> = vec![10, 20, 30];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
    // On LE with aligned buffer, should borrow directly
    if cfg!(target_endian = "little") {
      assert!(
        matches!(result, Cow::Borrowed(_)),
        "expected Cow::Borrowed on LE-aligned buffer"
      );
    }
  }

  #[test]
  fn read_scalar_array_unaligned_returns_owned() {
    // Build a buffer where the i32 data starts at an odd offset.
    let values: Vec<i32> = vec![42, 99];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let aligned_bytes = writer.into_bytes();
    // Prepend 1 byte to misalign the data
    let mut buf = vec![0u8];
    buf.extend_from_slice(&aligned_bytes);
    let mut reader = BebopReader::new(&buf[1..]);
    // The reader's buffer now starts at an odd address (1 byte into `buf`),
    // so the i32 data at offset 4 from the reader's start is at an odd
    // address overall. Alignment check should fail.
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
    // On LE, unaligned should fall back to Cow::Owned
    if cfg!(target_endian = "little") {
      // The buf slice starts 1 byte in, so pos=0 means ptr is buf.as_ptr()+1.
      // After reading 4-byte count, pos=4, data ptr = buf.as_ptr()+5 (odd).
      // For i32 (align=4), this is unaligned → Cow::Owned.
      assert!(
        matches!(result, Cow::Owned(_)),
        "expected Cow::Owned on unaligned buffer"
      );
    }
  }
```

- [ ] **Step 5: Add `Cow` import to test module**

In the `mod tests` block (line 355-357), add `use alloc::borrow::Cow;`:

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use alloc::borrow::Cow;
  use alloc::vec;
```

- [ ] **Step 6: Run runtime tests**

Run: `cd runtime && cargo test`

Expected: All 45 tests pass (43 existing updated + 2 new).

- [ ] **Step 7: Commit**

```bash
git add runtime/src/reader.rs
git commit -m "FD-023: read_scalar_array returns Cow with zero-copy borrow"
```

---

## Task 2: Generator — Lifetime Analysis

**Files:**
- Modify: `src/generator/type_mapper.rs:110` (make `is_bulk_scalar` pub)
- Modify: `src/generator/mod.rs:269-313` (`type_needs_lifetime`)

- [ ] **Step 1: Make `is_bulk_scalar` pub(crate)**

In `src/generator/type_mapper.rs:110`, change `fn is_bulk_scalar` to `pub(crate) fn is_bulk_scalar`:

```rust
pub(crate) fn is_bulk_scalar(kind: TypeKind) -> bool {
```

- [ ] **Step 2: Update `type_needs_lifetime` in mod.rs**

In `src/generator/mod.rs:278-288`, the `TypeKind::Array` branch currently checks for byte arrays and recurses. Add a bulk scalar check after the byte check. Replace lines 278-288:

```rust
      TypeKind::Array => {
        if let Some(ref elem) = td.array_element {
          // byte arrays need lifetime (Cow<'buf, [u8]>)
          if elem.kind == Some(TypeKind::Byte) {
            return true;
          }
          // bulk scalar arrays need lifetime (Cow<'buf, [T]>)
          if elem.kind.is_some_and(|k| type_mapper::is_bulk_scalar(k)) {
            return true;
          }
          self.type_needs_lifetime(elem)
        } else {
          false
        }
      }
```

- [ ] **Step 3: Run generator tests**

Run: `cargo test`

Expected: All 31 generator tests pass. Some may need updating if they test types with scalar arrays that now gain lifetimes.

- [ ] **Step 4: Commit**

```bash
git add src/generator/mod.rs src/generator/type_mapper.rs
git commit -m "FD-023: scalar arrays trigger lifetime in type_needs_lifetime"
```

---

## Task 3: Generator — Type Mapping and Cow Helpers

**Files:**
- Modify: `src/generator/type_mapper.rs` — functions: `rust_type`, `rust_type_owned`, `is_cow_field`, `into_owned_expression`, `into_borrowed_expression`

- [ ] **Step 1: Update `rust_type()` for bulk scalar arrays**

In `src/generator/type_mapper.rs:389-400`, after the byte array → `Cow<'buf, [u8]>` case (line 396), add a bulk scalar check before the generic `Vec<T>` fallback. Replace lines 394-399:

```rust
      // Byte arrays → Cow<'buf, [u8]>
      if elem.kind == Some(TypeKind::Byte) {
        return Ok("alloc::borrow::Cow<'buf, [u8]>".to_string());
      }
      // Bulk scalar arrays → Cow<'buf, [T]>
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("alloc::borrow::Cow<'buf, [{}]>", ty));
      }
      let inner = rust_type(elem, analysis)?;
      Ok(format!("alloc::vec::Vec<{}>", inner))
```

- [ ] **Step 2: Update `rust_type_owned()` for bulk scalar arrays**

In `src/generator/type_mapper.rs:321-331`, after the byte array → `Vec<u8>` case (line 327), add the bulk scalar case. Replace lines 326-330:

```rust
      if elem.kind == Some(TypeKind::Byte) {
        return Ok("alloc::vec::Vec<u8>".to_string());
      }
      // Bulk scalar arrays → Vec<T> (owned form of Cow<[T]>)
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("alloc::vec::Vec<{}>", ty));
      }
      let inner = rust_type_owned(elem, analysis)?;
      Ok(format!("alloc::vec::Vec<{}>", inner))
```

- [ ] **Step 3: Update `is_cow_field()` for bulk scalar arrays**

In `src/generator/type_mapper.rs:283-293`, extend the `TypeKind::Array` branch to also check for bulk scalar elements. Replace lines 288-292:

```rust
  match kind {
    TypeKind::String => true,
    TypeKind::Array => {
      if is_byte_array_cow_field(td) {
        return true;
      }
      td.array_element
        .as_ref()
        .is_some_and(|e| e.kind.is_some_and(|k| is_bulk_scalar(k)))
    }
    _ => false,
  }
```

- [ ] **Step 4: Update `into_owned_expression()` for bulk scalar arrays**

In `src/generator/type_mapper.rs:812-830`, in the `TypeKind::Array` branch, after the byte array case, add bulk scalar before the existing `type_needs_lifetime(elem)` check. Replace lines 818-830:

```rust
      // Cow<[u8]> → Cow::Owned(v.into_owned())
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!("alloc::borrow::Cow::Owned({}.into_owned())", value));
      }
      // Cow<[T]> for bulk scalars → Cow::Owned(v.into_owned())
      if elem.kind.is_some_and(|k| is_bulk_scalar(k)) {
        return Ok(format!("alloc::borrow::Cow::Owned({}.into_owned())", value));
      }
      // Vec of lifetime types → map into_owned
      if analysis.type_needs_lifetime(elem) {
        let inner = into_owned_expression(elem, "_e", analysis)?;
        Ok(format!(
          "{}.into_iter().map(|_e| {}).collect()",
          value, inner
        ))
      } else {
        Ok(value.to_string())
      }
```

- [ ] **Step 5: Update `into_borrowed_expression()` for bulk scalar arrays**

In `src/generator/type_mapper.rs:904-924`, in the `TypeKind::Array` branch, after the byte array case, add bulk scalar. Replace lines 909-924:

```rust
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!("alloc::borrow::Cow::Owned({})", value));
      }
      // Cow<[T]> for bulk scalars: Vec<T> → Cow::from(Vec)
      if elem.kind.is_some_and(|k| is_bulk_scalar(k)) {
        return Ok(format!("alloc::borrow::Cow::from({})", value));
      }
      if analysis.type_needs_lifetime(elem) {
        let inner = into_borrowed_expression(elem, "_e", analysis)?;
        if inner == "_e" {
          Ok(value.to_string())
        } else {
          Ok(format!(
            "{}.into_iter().map(|_e| {}).collect()",
            value, inner
          ))
        }
      } else {
        Ok(value.to_string())
      }
```

- [ ] **Step 6: Run generator tests**

Run: `cargo test`

Expected: All 31 generator tests pass (some tests may need updates if they emit scalar array types that now use Cow).

- [ ] **Step 7: Commit**

```bash
git add src/generator/type_mapper.rs
git commit -m "FD-023: emit Cow<[T]> types for bulk scalar arrays"
```

---

## Task 4: Regenerate Integration Tests

**Files:**
- Modify: `integration-tests/src/test_types.rs` (regenerated)

- [ ] **Step 1: Rebuild the generator**

Run: `cargo build`

Expected: Builds cleanly.

- [ ] **Step 2: Regenerate test_types.rs**

Run: `cd integration-tests && ./generate.sh`

Expected: Script succeeds. `test_types.rs` is updated with `Cow<'buf, [T]>` fields for scalar arrays, lifetime params on types that now need them.

- [ ] **Step 3: Verify regenerated code compiles**

Run: `cd integration-tests && cargo check`

Expected: Compiles. If there are test compilation errors, fix them in the integration test source (`integration-tests/tests/integration.rs`). Likely fix: `HalfPrecisionMessage` tests assign `msg.f16_arr = Some(vec![...])` which must become `Some(vec![...].into())` since the field is now `Option<Cow<'buf, [f16]>>`. Constructor calls via `::new(vec![...], ...)` should work automatically since `new()` uses `impl Into<Cow<...>>`.

- [ ] **Step 4: Run integration tests**

Run: `cd integration-tests && cargo test`

Expected: All 108+ integration tests pass. The round-trip tests work because Cow and Vec produce the same wire format.

- [ ] **Step 5: Commit**

```bash
git add integration-tests/src/test_types.rs
git commit -m "FD-023: regenerate test_types.rs with Cow scalar arrays"
```

---

## Task 5: Hand-Edit Benchmark Types

**Files:**
- Modify: `benchmarks/src/benchmark_types.rs`

The benchmark types file is hand-maintained and not regenerated. Types needing changes:

- `Order` — `Vec<i64>`, `Vec<i32>` fields → `Cow<'buf, [i64]>`, `Cow<'buf, [i32]>`; struct gains `<'buf>`
- `EmbeddingBf16` — `Vec<bf16>` → `Cow<'buf, [bf16]>`; gains `<'buf>`
- `EmbeddingF32` — `Vec<f32>` → `Cow<'buf, [f32]>`; gains `<'buf>`
- `TensorShard<'buf>` — `Vec<u32>`, `Vec<bf16>` → `Cow<'buf, [u32]>`, `Cow<'buf, [bf16]>` (already has lifetime)
- `InferenceResponse` — contains `Vec<EmbeddingBf16>` where `EmbeddingBf16` now has lifetime → `InferenceResponse<'buf>` with `Vec<EmbeddingBf16<'buf>>`

For each type:
1. Add `<'buf>` lifetime parameter if not already present
2. Change `Vec<T>` scalar array fields to `Cow<'buf, [T]>`
3. Update `new()` constructor parameter types: `Vec<T>` → `impl Into<Cow<'buf, [T]>>`
4. Add/update `into_owned()` method with `Cow::Owned(v.into_owned())` for each Cow field
5. Add `type XxxOwned = Xxx<'static>;` alias if not present
6. Update `BebopDecode` impl: `read_scalar_array` already returns `Cow` — no code change in decode
7. Update `BebopEncode` impl: `write_scalar_array::<T>(&self.field)` already works via Cow deref — no code change in encode

- [ ] **Step 1: Edit struct definitions and impls**

Apply the field type, lifetime, constructor, into_owned, and type alias changes to all affected types listed above.

- [ ] **Step 2: Verify benchmark crate compiles**

Run: `cd benchmarks && cargo check`

Expected: Compiles cleanly.

- [ ] **Step 3: Run benchmark tests**

Run: `cd benchmarks && cargo test`

Expected: All golden file tests (23) pass.

- [ ] **Step 4: Commit**

```bash
git add benchmarks/src/benchmark_types.rs
git commit -m "FD-023: update benchmark types with Cow scalar arrays"
```

---

## Task 6: Full Validation

- [ ] **Step 1: Run test.sh**

Run: `./test.sh`

Expected: All checks pass — formatting, compiler checks, clippy, unit tests (generator + runtime), integration tests (std + no_std), benchmark crate tests, golden file cross-language verification.

- [ ] **Step 2: Run benchmarks**

Run: `cd benchmarks && cargo bench`

Expected: Decode benchmarks for scalar-array-heavy scenarios (TensorShard, Embedding, Order, InferenceResponse) show significant improvement over pre-FD-023 numbers. Encode benchmarks should be unchanged.

- [ ] **Step 3: Commit any fixups**

If test.sh or benchmarks revealed issues, fix and commit.

```bash
git add -A
git commit -m "FD-023: fixups from full validation"
```
