# Autoresearch Ideas — Rust Decode Performance

## Summary
Total improvement: **42.5%** (20,062ns → 11,533ns baseline best)

## Optimizations Applied (Kept)
1. `#[inline(always)]` on all reader methods + `read_n<const N>` for single bounds check (-10.4%)
2. Optimized `read_byte`/`read_bool` (avoid checked_add, use get_unchecked) + `read_str`/`read_string` (combine bounds checks, use get_unchecked) (-1.4%)
3. `#[inline]` on all generated decode methods via code generator (-4.4%)
4. `Vec::with_capacity` and `HashMap::with_capacity` instead of new+try_reserve (-2.7%)
5. **`simdutf8` for SIMD-accelerated UTF-8 validation** (-28.2% — biggest single win)
6. `#[inline(always)]` on generated struct decode methods (-1.0%)
7. `#[inline(always)]` on generated enum decode methods (-1.5%)

## Tried and Discarded (18 experiments)
- `#[inline(always)]` on FixedScalar/BebopFlagBits trait impl methods (no effect)
- `#[inline]` on `from_bytes` + `read_raw_bytes` with get_unchecked (no effect)
- `#[cold]` on error paths (regression — unfavorable code layout)
- Single 12-byte read for timestamp/duration (no effect — not on hot path)
- Validated message boundary + combined next_message_tag (no improvement — aarch64 branch prediction already excellent)
- Split UTF-8 validation: std for short vs simdutf8 for long (extra branch hurts)
- Writer inline annotations + scalar array fallback with_capacity (code bloat)
- read_decode_array (direct trait dispatch instead of closure) (Rust monomorphization already inlines closures)
- simdutf8 default features (no improvement over aarch64_neon)
- Generated encode/encoded_size inline annotations (no decode improvement)
- simd_from_utf8 direct return (compiler already equivalent)
- Batch struct decode (ensure_available + unchecked reads for TextSpan) — within noise, only 1 struct qualified
- Thin LTO (no improvement — #[inline(always)] already handles cross-crate)
- read_cow_str convenience method (regression — extra function layer)
- `#[inline(always)]` on read_str (compiler already inlines with just #[inline])
- Remove checked_mul in read_scalar_array on 64-bit (compiler already optimizes)
- `#[inline(always)]` on validate_utf8 wrapper (already inlined)
- Message decode with local vars instead of Default+mutation (LLVM generates identical code)

## Remaining Bottlenecks (require structural changes)
- **HashMap SipHash** (JsonLarge ~1800ns): std HashMap uses SipHash. Switching to hashbrown (foldhash) requires public API change. Would need generated code + fixtures to use `bebop_runtime::HashMap`.
- **Recursive Vec allocations** (TreeDeep ~1600ns): Each node allocates a Vec. SmallVec<[T;1]> would help but changes public API.
- **UTF-8 validation floor** (DocumentLarge ~3000ns, ChunkedText ~2000ns): simdutf8 is already near-optimal. 146KB × ~13ns/KB = ~1900ns theoretical minimum. We're close.
- **Per-element overhead in arrays** (TreeWide ~880ns): 100 message decodes with tag loops. Fundamental overhead of the wire format.

## Plateau Reached
15 consecutive discards (runs 14–28) confirm the optimization plateau. All safe, API-compatible micro-optimizations have been exhausted. The compiler (LLVM on aarch64) is generating near-optimal code for the current patterns. Additional discarded experiments in session 4:
- Cached buf.len() as struct field (LLVM already hoists it into a register)
- #[inline(always)] on message/union decoders (code bloat regression in TreeWide)

## Future Ideas (require API or structural changes)
- **Switch to hashbrown HashMap unconditionally** (if API break is acceptable): Estimate ~30-40% reduction for JsonLarge/JsonSmall/DocumentSmall. Would require changing `bebop_runtime::HashMap` to `hashbrown::HashMap` and updating all consumer code.
- **Arena allocator for recursive types**: Custom allocator that batches small allocations. Would eliminate per-node Vec alloc in TreeDeep/TreeWide. Significant engineering effort.
- **Lazy UTF-8 validation**: Validate only on `.as_str()` access, not at decode time. Would eliminate UTF-8 cost from decode entirely. Major API change (field type would need to be `LazyStr` not `Cow<str>`).
- **SVE2 SIMD for UTF-8** (future ARM cores): Wider SIMD could improve large string validation beyond NEON's ~48 GB/s throughput.
