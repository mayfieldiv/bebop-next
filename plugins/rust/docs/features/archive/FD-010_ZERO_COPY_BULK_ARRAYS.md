# FD-010: Zero-Copy / Bulk Read for Primitive Arrays

**Status:** Complete
**Completed:** 2026-03-17
**Priority:** High
**Effort:** High (> 4 hours)
**Impact:** 30-60,000x faster for large scalar arrays (benchmarked 2026-03-16)

## Problem

The Rust runtime reads/writes arrays of fixed-size scalars element-by-element. On little-endian platforms, the wire format for scalar arrays is identical to the in-memory representation, so bulk memcpy or zero-copy borrows are possible. C does true zero-copy; Swift does bulk memcpy. Rust has zero-copy for strings and byte arrays (`Cow`) but not scalar arrays.

## Solution

Implement both runtime methods and generator integration:

**Runtime (reader.rs / writer.rs):**
- `read_scalar_array<T: FixedScalar>() -> Vec<T>` — bulk memcpy on LE, per-element on BE
- `read_scalar_slice<T: FixedScalar>() -> Cow<'a, [T]>` — zero-copy borrow when aligned on LE
- `write_scalar_array<T: FixedScalar>(&[T])` — bulk write on LE

**Generator (type_mapper.rs + LifetimeAnalysis):**
- Map `array[int32]` to `Cow<'buf, [i32]>` (when containing type has lifetime)
- Update `type_needs_lifetime()` to return true for scalar arrays
- Use `read_scalar_slice` / `write_scalar_array` in read/write expressions

**Safety:**
- Alignment check via `ptr::align_offset()` — borrow when aligned, copy when not
- Exclude `bool` from bulk path (invalid bit patterns cause UB)
- Use `checked_mul` for `count * elem_size` overflow protection

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `runtime/src/reader.rs` | MODIFY | Add `read_scalar_array`, `read_scalar_slice` |
| `runtime/src/writer.rs` | MODIFY | Add `write_scalar_array` |
| `runtime/src/traits.rs` | MODIFY | Document `FixedScalar` safety contract |
| `src/generator/mod.rs` | MODIFY | Update `LifetimeAnalysis` for scalar arrays |
| `src/generator/type_mapper.rs` | MODIFY | Scalar array read/write expressions |

## Verification

- Round-trip tests for `i32`, `f64`, `u8` arrays via both methods
- Empty array works
- `bool` array still uses per-element path
- Enormous count triggers error, not panic
- Unaligned buffer falls back to `Cow::Owned`
- Benchmark: throughput improvement for 10K-element arrays

## Benchmark Data (2026-03-16)

Rust vs C comparison for scenarios dominated by scalar arrays:

| Scenario | Encode | Decode | Array size |
|----------|--------|--------|------------|
| TensorShardLarge | 539x slower | 62,615x slower | 368K bf16 |
| Embedding1536 | 35x | 569x | 1536 bf16 |
| EmbeddingBatch | 33x | 504x | 8×1536 bf16 |
| InferenceResponse | 34x | 205x | 4×768 bf16 |
| OrderLarge | 23x | 50x | 100 i64 + 100 i32 |

The write path (`write_scalar_array`) is the easy win — no API changes, just
`extend_from_slice` on LE. The read path (`Cow` slices) is harder because it
changes generated type signatures and lifetime analysis.

## Source

Migrated from `../../issues/bulk-array-optimization.md` and `../../issues/plans/zero-copy-arrays.md`
