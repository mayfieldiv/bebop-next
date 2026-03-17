# FD-023: Zero-Copy Scalar Array Decode

**Status:** Open
**Priority:** Medium
**Effort:** High (> 4 hours)
**Impact:** Eliminate allocation + memcpy on decode for scalar arrays, reaching parity with C's zero-copy pointer-into-buffer approach

## Problem

After FD-010 (bulk scalar array write/read), the encode path reaches near-parity with C via bulk `memcpy`. However, the decode path still allocates a `Vec<T>` and copies — C instead returns a borrowed pointer directly into the wire buffer with `capacity=0` to mark it as a view.

Benchmark projections after FD-010 bulk memcpy:

| Scenario | C decode (ns) | Rust decode (est.) | Gap | Root cause |
|----------|---------------|-------------------|-----|------------|
| TensorShardLarge | 7 | ~500 | ~70x | alloc 737KB + memcpy |
| Embedding1536 | 3 | ~30 | ~10x | alloc 3KB + memcpy |
| EmbeddingBatch | 26 | ~200 | ~8x | 8 × (alloc + memcpy) |

The C runtime achieves near-zero-cost decode by returning `{.data = reader_ptr, .length = count, .capacity = 0}` — a borrowed view with no allocation or copy.

## Solution

Change generated scalar array fields from `Vec<T>` to `Cow<'buf, [T]>` for types where the borrowed path is safe, and add a `read_scalar_slice` method to the runtime that returns a zero-copy borrow on LE-aligned buffers.

**Runtime (reader.rs):**
- `read_scalar_slice<T: BulkScalar>() -> Cow<'a, [T]>` — checks alignment via `ptr::align_offset()`, returns `Cow::Borrowed` when aligned on LE, falls back to `Cow::Owned` (bulk memcpy) otherwise

**Generator (type_mapper.rs + LifetimeAnalysis):**
- Map `array[int32]` → `Cow<'buf, [i32]>` in generated struct fields (when containing type has lifetime)
- Update `type_needs_lifetime()` to return true for structs containing scalar arrays
- Use `read_scalar_slice` in read expressions, `write_scalar_array` in write expressions (from FD-010)
- Update `into_owned()` to convert `Cow::Borrowed` → `Cow::Owned`

**Safety:**
- Alignment check via `ptr::align_offset()` — borrow when aligned, alloc+copy when not
- Exclude `bool` from zero-copy path (invalid bit patterns)
- On big-endian, always fall back to `Cow::Owned` with per-element decode

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/reader.rs` | MODIFY | Add `read_scalar_slice<T: BulkScalar>() -> Cow<'a, [T]>` |
| `src/generator/mod.rs` | MODIFY | Update `LifetimeAnalysis` — scalar arrays now need lifetime |
| `src/generator/type_mapper.rs` | MODIFY | Emit `Cow<'buf, [T]>` for scalar array fields, use `read_scalar_slice` |

## Verification

- Round-trip tests for all scalar array types via both Cow::Borrowed and Cow::Owned paths
- Aligned buffer → Cow::Borrowed, unaligned → Cow::Owned
- `into_owned()` converts borrowed to owned correctly
- Empty array returns Cow::Borrowed(&[])
- `bool` array still uses per-element path (not zero-copy)
- Benchmark: decode throughput approaches C's zero-copy numbers

## Related

- FD-010: Zero-Copy Bulk Arrays (prerequisite — adds `BulkScalar` trait and bulk memcpy write/read)
- Existing `Cow<'buf, str>` / `Cow<'buf, [u8]>` patterns in generated code (strings and byte arrays already use this approach)
