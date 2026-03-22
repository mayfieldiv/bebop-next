# Autoresearch Ideas — Rust Decode Performance

## Tried and Kept
- `#[inline(always)]` on all reader methods + `read_n<const N>` for single bounds check
- Optimized `read_byte`/`read_bool` (avoid checked_add, use get_unchecked)
- Optimized `read_str`/`read_string` (combine bounds checks, use get_unchecked)
- `#[inline]` on all generated decode methods via code generator

## Tried and Discarded
- `#[inline(always)]` on FixedScalar/BebopFlagBits trait impl methods (no measurable effect)
- `#[inline]` on `from_bytes` + `read_raw_bytes` with get_unchecked (no measurable effect)

## Deferred Ideas
- **Switch HashMap to hashbrown unconditionally**: Would use foldhash instead of SipHash. ~2-5x faster hashing. Blocked by public API change (fixtures use std::collections::HashMap). Could work if generated code imports `bebop_runtime::HashMap` for construction too.
- **SIMD UTF-8 validation**: Use `simdutf8` crate for faster validation in `read_str`/`read_string`. Could help DocumentLarge (146KB string). std `from_utf8` may already use SIMD on recent Rust.
- **Batch bounds checking for fixed-size structs**: Instead of checking bounds per-field (3 checks for TextSpan: u32+u32+u8 = 9 bytes), check once for the whole struct. Requires generator changes.
- **Reduce HashMap overhead for small maps**: For map fields with 1-3 entries, a Vec of tuples would be faster than HashMap. Needs API/type change.
- **Arena allocation for recursive types**: TreeDeep allocates Vec per node. Arena allocation could reduce allocator pressure.
- **read_timestamp/read_duration as single 12-byte read**: Use `read_n::<12>()` instead of two separate reads. Small win.
- **Optimize enum decode**: TryFrom match could be replaced with bounds check + transmute for sequential enums.
- **Profile-guided optimization (PGO)**: Not applicable to library code, but worth noting.
- **Add `#[inline]` to generated encode methods**: Would help encode benchmarks (secondary metric).
