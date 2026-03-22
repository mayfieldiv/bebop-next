# Autoresearch Ideas — Rust Decode Performance

## Summary
Total improvement: **~44.5%** (20,062ns → ~11,140ns)

## Optimizations Applied (Kept)
1. `#[inline]` on all reader methods + `read_n<const N>` with safe ensure+try_into
2. `#[inline]` on all generated decode methods (struct, message, union)
3. `#[inline(always)]` on generated enum decode methods only
4. `Vec::with_capacity` and `HashMap::with_capacity` instead of new
5. **`simdutf8` for SIMD-accelerated UTF-8 validation** (-28.2% — biggest single win)
6. **`hashbrown::HashMap` (foldhash) always** — breaking API change (JsonLarge -17.7%)
7. Simplified `ensure()` using wrapping_sub instead of checked_add
8. `read_array` ptr::write + set_len instead of push (TreeWide -13%)
9. Safe code throughout — removed get_unchecked, from_utf8_unchecked, ptr::read_unaligned

## Confirmed Essential (Cannot Simplify)
- **simdutf8**: +46% regression without it
- **Vec/HashMap::with_capacity**: +16% regression without it
- **#[inline(always)] on enum decode**: +3.9% regression when removed
- **#[inline] on message/union decode**: +5.4% regression without it
- **Dedicated read_byte/read_bool** (not read_n::<1>): +5.8% regression

## Confirmed Unnecessary (Ablated Away)
- get_unchecked on read_byte/read_bool/read_str/read_string
- from_utf8_unchecked (simdutf8 returns &str directly)
- ptr::read_unaligned in read_n (try_into().unwrap() is identical)
- #[inline(always)] on reader.rs (plain #[inline] is marginally better)
- #[inline(always)] on struct decode (plain #[inline] is equivalent)
- Inlined u32 read in read_str (LLVM already inlines the chain)
- read_map /2 division optimization (negligible one-time cost)

## Remaining Bottlenecks (immovable or require structural changes)
- **UTF-8 validation floor** (DocumentLarge ~2850ns, ChunkedText ~2000ns): simdutf8 is near-optimal
- **Recursive Vec allocations** (TreeDeep ~1640ns): Each node allocates a Vec. SmallVec<[T;1]> would help but changes public API
- **HashMap operations** (JsonLarge ~1500ns): Already using hashbrown. Remaining cost is hashing + probing
- **Per-element overhead in arrays** (TreeWide ~760ns): Optimized with ptr::write
- **String array decode** (LLMChunkLarge ~1340ns): Dominated by UTF-8 validation + alloc

## Future Ideas (require API or structural changes)
- **SmallVec for arrays in recursive types**: Estimate ~1000ns reduction for TreeDeep. Feature-gated.
- **Arena allocator**: Batch all decode allocations. Major engineering effort.
