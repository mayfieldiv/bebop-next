# FD-023: Zero-Copy Scalar Array Decode — Design Spec

## Overview

Replace the bulk-memcpy `read_scalar_array` (returns `Vec<T>`) with a zero-copy implementation that returns `Cow<'buf, [T]>`, borrowing directly from the wire buffer when alignment permits. Update the generator to emit `Cow<'buf, [T]>` field types, lifetime analysis, and into_owned handling for all `BulkScalar` array fields.

This is the natural successor to FD-010 (bulk memcpy encode/decode). FD-010 eliminated per-element loops; FD-023 eliminates the allocation and copy entirely on aligned LE buffers.

## Runtime Changes

### `read_scalar_array` signature change

**File:** `runtime/src/reader.rs`

Replace the existing `read_scalar_array` implementation. Same name, new return type:

```rust
pub fn read_scalar_array<T: BulkScalar>(&mut self) -> Result<Cow<'a, [T]>>
```

**Logic:**

1. Read `u32` count. If zero, return `Cow::Borrowed(&[])` immediately.
2. `checked_mul(count, size_of::<T>())` for byte length; `ensure(byte_len)` for bounds.
3. On **LE + aligned** (`self.buf[self.pos..].as_ptr().align_offset(align_of::<T>()) == 0`):
   - `Cow::Borrowed(slice::from_raw_parts(ptr as *const T, count))`
   - Advance `self.pos += byte_len`
4. On **LE + unaligned**:
   - Allocate `Vec<T>` via `try_reserve`, bulk `copy_nonoverlapping`, return `Cow::Owned(vec)`
5. On **BE**:
   - Per-element `T::read_from()` into Vec, return `Cow::Owned(vec)`

**Safety invariants** (same as FD-010 plus alignment):

- `T: BulkScalar` guarantees all bit patterns are valid and `size_of::<T>()` equals wire element size
- On LE, wire byte order equals memory layout
- Alignment check via `align_offset` ensures the borrowed pointer is valid for `T` reads
- `ensure()` guarantees the slice doesn't extend past the buffer
- The borrowed slice's lifetime is tied to `'a` (the buffer lifetime), which is correct since the reader borrows the buffer

### No other runtime changes

- `write_scalar_array` is unchanged (takes `&[T]`, Cow derefs to that)
- `BulkScalar` trait is unchanged
- `read_byte_slice` / `write_byte_array` remain as-is (separate `TypeKind::Byte` path)

## Generator Changes

### Lifetime Analysis (`src/generator/mod.rs`)

**`type_needs_lifetime()`** — In the `TypeKind::Array` branch, after the existing `TypeKind::Byte` check, add: if the element type is a bulk scalar (check via `is_bulk_scalar` from type_mapper), return `true`.

This causes structs and messages containing scalar arrays to gain `<'buf>` lifetime parameters through the fixpoint iteration, just like structs containing strings or byte arrays. For example, `HalfPrecisionArrays` (struct) and `HalfPrecisionMessage` (message) both gain lifetimes.

### Type Mapping (`src/generator/type_mapper.rs`)

**`rust_type()`** — In the `TypeKind::Array` branch, after the byte array → `Cow<'buf, [u8]>` case, add: if element is bulk scalar, return `alloc::borrow::Cow<'buf, [T]>` where T is the scalar Rust type (e.g. `i32`, `f64`, `bf16`).

**`rust_type_owned()`** — Same position: bulk scalar arrays map to `alloc::vec::Vec<T>` (the owned form).

**`is_cow_field()`** — Extend to return `true` for bulk scalar array fields (not just strings and byte arrays). This drives constructor patterns and serde attribute generation.

### Read Expression

No change to emitted code. The generator already emits `reader.read_scalar_array::<T>()` — the runtime's return type changes from `Vec<T>` to `Cow<'a, [T]>` transparently.

### Write Expression

No change. `write_scalar_array::<T>(&self.field)` works because `Cow<[T]>` derefs to `&[T]`.

### `into_owned_expression()`

In the `TypeKind::Array` branch, add bulk scalar case: emit `alloc::borrow::Cow::Owned(v.into_owned())`, mirroring the byte array pattern.

### `into_borrowed_expression()`

Add bulk scalar case: emit `alloc::borrow::Cow::from(v)` to convert `Vec<T>` into `Cow<'buf, [T]>` in constructors.

### `encoded_size` expression

No change. `Cow<[T]>` derefs to `&[T]`, so `.len()` works as before.

### Serde

No special handling needed. `Cow<'a, [i32]>` serializes/deserializes as a sequence out of the box. Unlike byte arrays (which need `#[serde(with = "serde_cow_bytes")]`), scalar Cow slices work with default serde behavior. `#[serde(borrow)]` is not added since serde can only borrow `str` and `[u8]` — for other types it always deserializes to `Cow::Owned`, which is correct.

## Files Modified

| File | Change |
|------|--------|
| `runtime/src/reader.rs` | Replace `read_scalar_array` impl: `Vec<T>` → `Cow<'a, [T]>` with alignment check |
| `src/generator/mod.rs` | `type_needs_lifetime()`: bulk scalar arrays trigger lifetime |
| `src/generator/type_mapper.rs` | `rust_type`, `rust_type_owned`, `is_cow_field`, `into_owned_expression`, `into_borrowed_expression` |
| `integration-tests/src/test_types.rs` | Regenerated (scalar array structs gain lifetimes + Cow fields) |
| `benchmarks/src/benchmark_types.rs` | Hand-edited (same Cow treatment for benchmark types) |

## Testing

### New runtime tests

- Aligned buffer → verify `Cow::Borrowed` (check discriminant or pointer identity)
- Unaligned buffer → verify `Cow::Owned` fallback
- Empty array → `Cow::Borrowed(&[])`
- Round-trip for representative types: i32, f64, bf16
- Existing overflow/malicious count tests updated for new signature

### Integration tests

Existing round-trip tests pass after regeneration. Construction sites updated with `.into()` where needed.

### Benchmarks

Decode benchmarks for TensorShard, Embedding, InferenceResponse should approach C's near-zero numbers (eliminating alloc+memcpy).

## Non-Goals

- Changing the byte array path (`TypeKind::Byte`) — stays separate, works fine as-is
- Zero-copy for non-scalar arrays (strings, defined types) — already handled or not applicable
- Big-endian zero-copy — fundamentally impossible without byte-swapping, always falls back to owned
