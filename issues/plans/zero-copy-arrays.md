# Plan: Zero-Copy / Bulk Read for Primitive Arrays

**Status:** Draft
**Depends on:** None
**Blocks:** [oom-handling.md] (bulk reads require careful OOM consideration)

## Problem Statement

The Rust runtime reads arrays of fixed-size scalars (e.g., `array[int32]`, `array[float64]`) by looping element-by-element, allocating a `Vec<T>` and calling the individual read method for each element. This is orders of magnitude slower than necessary because:

1. The wire format for arrays of little-endian primitives on little-endian platforms is **identical** to the in-memory representation
2. The C plugin does true zero-copy for primitive arrays (pointer into the read buffer)
3. The Swift plugin does bulk `memcpy` reads via `readLengthPrefixedArray`

The Rust plugin has zero-copy for strings (`Cow<'buf, str>`) and byte arrays (`Cow<'buf, [u8]>`), but not for any other primitive arrays.

## Current Code Analysis

### `reader.rs:199-209` — `read_array()`

```rust
pub fn read_array<T>(
  &mut self,
  mut read_elem: impl FnMut(&mut Self) -> Result<T>,
) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  let mut items = Vec::with_capacity(count);
  for _ in 0..count {
    items.push(read_elem(self)?);
  }
  Ok(items)
}
```

This is the generic array reader. It takes a closure for reading each element, allocates a `Vec`, and loops. No bulk path.

### `reader.rs:226-232` — `read_fixed_array()`

```rust
pub fn read_fixed_array<T: FixedScalar, const N: usize>(&mut self) -> Result<[T; N]> {
  let mut arr = [T::default(); N];
  for item in &mut arr {
    *item = T::read_from(self)?;
  }
  Ok(arr)
}
```

Fixed-size arrays (e.g., `int32[5]`) also loop per-element.

### `writer.rs:174-179` — `write_array()`

```rust
pub fn write_array<T>(&mut self, items: &[T], mut write_elem: impl FnMut(&mut Self, &T)) {
  self.write_u32(items.len() as u32);
  for item in items {
    write_elem(self, item);
  }
}
```

Writing also loops element-by-element.

### `writer.rs:200-204` — `write_fixed_array()`

```rust
pub fn write_fixed_array<T: FixedScalar, const N: usize>(&mut self, arr: &[T; N]) {
  for item in arr {
    item.write_to(self);
  }
}
```

### `traits.rs:237-375` — `FixedScalar` Trait

The `FixedScalar` trait has per-element `read_from()`/`write_to()`. Each implementation does bounds checking, slice copying, and byte-order conversion individually.

### `type_mapper.rs` — Generated Read/Write Expressions

The generated code for reading an `array[int32]` produces:

```rust
reader.read_array(|reader| reader.read_i32())
```

This chains through `read_array()` → `read_i32()` → `ensure()` + `advance()` + `from_le_bytes()` per element.

## Design Options

### Option A: Zero-Copy Borrowed Slice

On little-endian platforms, `&[i32]` has the same memory layout as a sequence of `i32` in little-endian wire format. We can borrow directly from the buffer:

```rust
/// Read an array of fixed-size scalars as a borrowed slice (zero-copy on LE).
///
/// Returns `Cow::Borrowed` on little-endian targets and `Cow::Owned` on big-endian.
pub fn read_scalar_slice<T: FixedScalar>(&mut self) -> Result<Cow<'a, [T]>>
where
  [T]: bytemuck::AnyBitPattern,  // or equivalent safety requirement
{
  let count = self.read_u32()? as usize;
  let byte_len = count * core::mem::size_of::<T>();
  self.ensure(byte_len)?;
  let slice = &self.buf[self.pos..self.pos + byte_len];
  self.pos += byte_len;

  #[cfg(target_endian = "little")]
  {
    // SAFETY: FixedScalar types (excluding bool) are primitive numerics where
    // all bit patterns are valid and the wire format is little-endian, matching
    // native memory layout. However, &[u8] only guarantees alignment 1, and
    // creating a misaligned &[T] is undefined behavior even without dereferencing
    // (per Rust reference: "references must be aligned").
    //
    // We use ptr::align_offset() to check alignment at runtime. This function
    // is guaranteed to return a correct result (not usize::MAX) for pointers to
    // allocated objects since Rust 1.78.0 (stabilized in rust-lang/rust#121201).
    // For earlier Rust versions, align_offset may return usize::MAX as a
    // conservative "unknown" answer, which our check handles correctly by
    // falling through to the copy path.
    let ptr = slice.as_ptr();
    if ptr.align_offset(core::mem::align_of::<T>()) == 0 {
      // Aligned: zero-copy borrow
      let typed_slice = unsafe {
        core::slice::from_raw_parts(ptr as *const T, count)
      };
      Ok(Cow::Borrowed(typed_slice))
    } else {
      // Unaligned: must copy (same as bytemuck/zerocopy behavior — both
      // libraries check alignment before casting and return errors on
      // misalignment rather than invoking UB)
      Ok(Cow::Owned(copy_le_scalars::<T>(slice, count)))
    }
  }
  #[cfg(not(target_endian = "little"))]
  {
    Ok(Cow::Owned(copy_le_scalars::<T>(slice, count)))
  }
}
```

**Alignment summary:**

- `&[u8]` (the read buffer) only guarantees alignment of 1 byte
- Creating a misaligned `&[T]` is instant UB, even without dereferencing the data
- `ptr::align_offset()` is the correct way to check alignment at runtime; it is guaranteed to return an accurate result (not `usize::MAX`) since Rust 1.78.0 for pointers to allocated objects
- Both `bytemuck` and `zerocopy` use this same pattern: check alignment, return error on misalignment
- When the buffer happens to be aligned (common for heap-allocated buffers), we get true zero-copy; otherwise we fall back to memcpy

### Option B: Bulk Copy (Always Allocate, but Fast)

Skip the zero-copy complexity and just do a bulk `memcpy` on LE or per-element byte-swap on BE:

```rust
pub fn read_scalar_array<T: FixedScalar>(&mut self) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  let byte_len = count * core::mem::size_of::<T>();
  self.ensure(byte_len)?;

  let mut vec: Vec<T> = Vec::with_capacity(count);
  #[cfg(target_endian = "little")]
  {
    // SAFETY: we're copying byte_len bytes into uninitialized Vec capacity.
    // T: FixedScalar guarantees all bit patterns are valid.
    unsafe {
      core::ptr::copy_nonoverlapping(
        self.buf[self.pos..].as_ptr(),
        vec.as_mut_ptr() as *mut u8,
        byte_len,
      );
      vec.set_len(count);
    }
  }
  #[cfg(not(target_endian = "little"))]
  {
    for _ in 0..count {
      vec.push(T::read_from(self)?);
    }
  }

  self.pos += byte_len;
  Ok(vec)
}
```

This is simpler than Option A (no alignment check, no `Cow`) and still provides massive speedup (one memcpy vs N individual reads). `copy_nonoverlapping` has no alignment requirements on the *source* pointer when copying to/from `*mut u8`.

### Option C: `Cow<'buf, [T]>` in Generated Types

Change the generated type for `array[int32]` from `Vec<i32>` to `Cow<'buf, [i32]>`, enabling true zero-copy in the type system. This mirrors how `string` → `Cow<'buf, str>` and `byte[]` → `Cow<'buf, [u8]>` already work.

This is a natural extension of the existing pattern — strings and byte arrays already use `Cow` for zero-copy, and scalar arrays have the same wire-format-matches-memory-layout property on little-endian platforms. The changes required are well-scoped:

1. **`LifetimeAnalysis::type_needs_lifetime()`** — return `true` for arrays of scalars (currently only returns true for strings, byte arrays, and types containing them)
2. **`type_mapper::rust_type()`** — map `array[int32]` → `Cow<'buf, [i32]>` instead of `Vec<i32>`
3. **`reader.rs`** — add `read_scalar_slice()` → `Cow<'buf, [T]>` (Option A's reader method)
4. **`writer.rs`** — add `write_scalar_slice()` that accepts `&[T]` (works for both `Cow::Borrowed` and `Cow::Owned` via deref)
5. **`into_owned()` expressions** — add `Cow::into_owned()` for scalar array fields
6. **`type_mapper.rs`** — update `read_expression()` and `write_expression()` for scalar arrays

This follows the same pattern already established for strings and byte arrays. The lifetime propagation through `LifetimeAnalysis` already handles the fixpoint iteration for nested types, so a struct containing an `array[int32]` field would correctly gain a `'buf` lifetime parameter.

## Recommended Approach: Options B + C Together

Rather than phasing B first and C later, both can be implemented together since they share the same runtime methods and C is a natural extension of existing patterns:

### Runtime Changes (shared by B and C)

#### 1. `reader.rs` — Add both `read_scalar_array()` and `read_scalar_slice()`

```rust
/// Read a length-prefixed array of fixed-size scalars using bulk copy.
/// Returns an owned Vec<T>.
///
/// On little-endian platforms, this copies the entire byte range at once
/// instead of reading element-by-element. On big-endian platforms, it
/// falls back to per-element reads with byte-order conversion.
pub fn read_scalar_array<T: FixedScalar>(&mut self) -> Result<Vec<T>> {
  let count = self.read_u32()? as usize;
  if count == 0 {
    return Ok(Vec::new());
  }
  let elem_size = core::mem::size_of::<T>();
  let byte_len = count.checked_mul(elem_size)
    .ok_or(DecodeError::UnexpectedEof { needed: usize::MAX, available: self.remaining() })?;
  self.ensure(byte_len)?;

  let mut vec: Vec<T> = Vec::with_capacity(count);

  #[cfg(target_endian = "little")]
  {
    // SAFETY: FixedScalar types (excluding bool) are primitive numerics.
    // All bit patterns are valid for these types. We're copying exactly
    // byte_len bytes from a validated buffer range. copy_nonoverlapping
    // has no alignment requirement when the destination is cast to *mut u8.
    unsafe {
      core::ptr::copy_nonoverlapping(
        self.buf.as_ptr().add(self.pos),
        vec.as_mut_ptr() as *mut u8,
        byte_len,
      );
      vec.set_len(count);
    }
    self.pos += byte_len;
  }

  #[cfg(not(target_endian = "little"))]
  {
    for _ in 0..count {
      vec.push(T::read_from(self)?);
    }
  }

  Ok(vec)
}

/// Read a length-prefixed array of fixed-size scalars, borrowing from the
/// buffer when possible (zero-copy on little-endian with aligned data).
///
/// Returns `Cow::Borrowed` when the buffer data is correctly aligned for T
/// on little-endian platforms. Returns `Cow::Owned` (via bulk memcpy) otherwise.
pub fn read_scalar_slice<T: FixedScalar>(&mut self) -> Result<Cow<'a, [T]>>
where
  T: Clone,  // required by Cow<[T]>
{
  let count = self.read_u32()? as usize;
  if count == 0 {
    return Ok(Cow::Borrowed(&[]));
  }
  let elem_size = core::mem::size_of::<T>();
  let byte_len = count.checked_mul(elem_size)
    .ok_or(DecodeError::UnexpectedEof { needed: usize::MAX, available: self.remaining() })?;
  self.ensure(byte_len)?;

  let slice = &self.buf[self.pos..self.pos + byte_len];
  self.pos += byte_len;

  #[cfg(target_endian = "little")]
  {
    let ptr = slice.as_ptr();
    if ptr.align_offset(core::mem::align_of::<T>()) == 0 {
      // SAFETY: T is FixedScalar (primitive numeric, all bit patterns valid).
      // We verified alignment above. The borrow is valid for lifetime 'a
      // (the buffer's lifetime).
      let typed_slice = unsafe {
        core::slice::from_raw_parts(ptr as *const T, count)
      };
      Ok(Cow::Borrowed(typed_slice))
    } else {
      // Unaligned: bulk copy into a new Vec
      let mut vec = Vec::with_capacity(count);
      unsafe {
        core::ptr::copy_nonoverlapping(
          slice.as_ptr(),
          vec.as_mut_ptr() as *mut u8,
          byte_len,
        );
        vec.set_len(count);
      }
      Ok(Cow::Owned(vec))
    }
  }

  #[cfg(not(target_endian = "little"))]
  {
    let mut vec = Vec::with_capacity(count);
    // Re-create a temporary reader over the slice for per-element byte swap
    // (self.pos already advanced, so use the captured slice)
    for i in 0..count {
      let offset = i * elem_size;
      let elem_bytes = &slice[offset..offset + elem_size];
      // Use FixedScalar::read_from with a sub-reader
      vec.push(T::read_from(&mut BebopReader::new(elem_bytes))?);
    }
    Ok(Cow::Owned(vec))
  }
}
```

#### 2. `writer.rs` — Add `write_scalar_array()` method

```rust
/// Write a length-prefixed array of fixed-size scalars using bulk copy.
pub fn write_scalar_array<T: FixedScalar>(&mut self, items: &[T]) {
  self.write_u32(items.len() as u32);
  if items.is_empty() {
    return;
  }

  #[cfg(target_endian = "little")]
  {
    let byte_len = items.len() * core::mem::size_of::<T>();
    // SAFETY: T is FixedScalar (primitive numeric). Reinterpreting as bytes
    // is safe because all bit patterns are valid and there's no padding.
    let bytes = unsafe {
      core::slice::from_raw_parts(items.as_ptr() as *const u8, byte_len)
    };
    self.buf.extend_from_slice(bytes);
  }

  #[cfg(not(target_endian = "little"))]
  {
    for item in items {
      item.write_to(self);
    }
  }
}
```

#### 3. `type_mapper.rs` — Update Read/Write Expressions

For types that use `Cow<'buf, [T]>` (Option C path):

```rust
// Read expression for array[int32] → Cow<'buf, [i32]>:
reader.read_scalar_slice::<i32>()

// Write expression:
writer.write_scalar_array(&self.values)  // Cow<[T]> derefs to &[T]
```

For types that use `Vec<T>` (Option B fallback, e.g., when the containing type doesn't have a lifetime):

```rust
// Read expression for array[int32] → Vec<i32>:
reader.read_scalar_array::<i32>()

// Write expression:
writer.write_scalar_array(&self.values)
```

#### 4. `LifetimeAnalysis` — Arrays of Scalars Need Lifetime

In `mod.rs`, update `type_needs_lifetime()` to return `true` when the type is an array of a fixed scalar. This causes the containing struct/message to gain a `'buf` lifetime parameter, which is required for `Cow<'buf, [T]>`.

#### 5. `traits.rs` — Add `FixedScalar` Safety Marker

Add a doc comment noting the safety contract:

```rust
/// Scalar types that can appear in fixed-size arrays.
///
/// # Safety
///
/// Implementors must guarantee that:
/// - `size_of::<Self>()` matches the wire format byte count
/// - All bit patterns of that size represent valid values
/// - The wire format is little-endian, matching the memory representation
///   on little-endian platforms
///
/// These invariants are used by `read_scalar_array()` and `read_scalar_slice()`
/// to perform bulk copies and zero-copy borrows.
pub trait FixedScalar: Copy + Default {
  fn read_from<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError>;
  fn write_to(self, writer: &mut BebopWriter);
}
```

Consider making `FixedScalar` an `unsafe trait` since its safety contract is now load-bearing.

**Safety justification:** `FixedScalar` is only implemented for `bool`, `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`, `f16`, `bf16`, `f32`, `f64`. All of these:
- Have no padding bytes
- Accept all bit patterns (including NaN for floats)
- Are stored in little-endian wire format, matching little-endian memory layout

Note: `bool` is technically only guaranteed to be 0 or 1, but the wire format only produces 0 or 1 as well. However, if a malicious buffer contains `bool` value 2, the bulk copy would create an invalid `bool`. We should **exclude `bool`** from the bulk path or add validation.

## Cost-Benefit Analysis

### Benefits

- **Performance:** ~10-100x faster for large arrays of primitives (one memcpy or zero-copy borrow vs N individual reads/writes)
- **Consistency:** Scalar arrays get the same `Cow` zero-copy treatment as strings and byte arrays
- **Minimal generator change:** `LifetimeAnalysis` and `type_mapper.rs` changes follow established patterns
- **Runtime methods usable independently:** `read_scalar_array()` and `read_scalar_slice()` work regardless of generated code

### Costs

- **Unsafe code:** ~15 lines of `unsafe` in reader.rs and writer.rs, requiring careful review
- **Platform-specific:** The optimization only fires on little-endian. Big-endian falls back to current behavior (zero regression)
- **Bool edge case:** `bool` should be excluded from bulk copy to avoid UB from invalid values, or a post-copy validation pass is needed
- **Overflow check:** `count * elem_size` can overflow for malicious inputs — must use `checked_mul`
- **Lifetime propagation:** Types containing scalar arrays gain a `'buf` lifetime, which is a type-level change (but consistent with strings/byte arrays)

### Generated Code Size Impact

- Current: `reader.read_array(|reader| reader.read_i32())` (42 chars)
- Proposed: `reader.read_scalar_slice::<i32>()` (35 chars)
- **Net:** Slightly smaller per array field

### Runtime Code Size Impact

- New: `read_scalar_array()` ~30 lines, `read_scalar_slice()` ~40 lines, `write_scalar_array()` ~15 lines
- These are generic methods monomorphized per scalar type
- Trade-off: remove per-type closure instantiations (each `|reader| reader.read_i32()` closure is a separate function)

## Bool Safety Analysis

For `bool`, Rust requires the value to be 0 or 1. Bulk copying from a buffer could yield `bool` values of 2, 3, etc., which is UB. Options:

1. **Exclude `bool` from bulk path** — simplest, minimal performance impact (bool arrays are rare)
2. **Post-copy validation** — iterate and check each byte is 0 or 1 after copying
3. **Use `u8` then transmute** — doesn't help, same UB concern

**Recommendation:** Exclude `bool` from the bulk path. The `is_fixed_scalar()` check in `type_mapper.rs` already knows the element type, so this is trivial:

```rust
fn can_bulk_copy(kind: TypeKind) -> bool {
  matches!(kind,
    TypeKind::Byte | TypeKind::Int8 |
    TypeKind::Uint16 | TypeKind::Int16 |
    TypeKind::Uint32 | TypeKind::Int32 |
    TypeKind::Uint64 | TypeKind::Int64 |
    TypeKind::Float16 | TypeKind::Bfloat16 |
    TypeKind::Float32 | TypeKind::Float64
  )
  // Note: Bool excluded, Uint128/Int128 excluded (rare, verify layout)
}
```

## Test Plan

1. Unit test: `read_scalar_array` for `i32`, `f64`, `u8` — verify output matches per-element read
2. Unit test: `read_scalar_slice` for `i32`, `f64` — verify output matches per-element read, check `Cow::Borrowed` vs `Cow::Owned`
3. Unit test: `write_scalar_array` for `i32`, `f64` — verify output matches per-element write
4. Unit test: empty array — verify both methods return empty result
5. Unit test: `bool` array — verify still uses per-element path (no bulk copy)
6. Round-trip test: encode with `write_scalar_array`, decode with both `read_scalar_array` and `read_scalar_slice`
7. Overflow test: enormous `count` value triggers error, not panic
8. Integration test: schema with `array[int32]` field round-trips correctly with `Cow<'buf, [i32]>` type
9. Alignment test: verify unaligned buffer falls back to `Cow::Owned` (not UB)
10. Benchmark: measure throughput improvement for 10K-element i32 array
