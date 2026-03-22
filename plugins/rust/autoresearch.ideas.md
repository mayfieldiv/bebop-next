# Autoresearch Ideas — Rust Decode Performance

## Summary
Total improvement: **~44%** (20,062ns → ~11,225ns)

## Optimizations Applied (Kept)
1. `#[inline]` on all reader methods + `read_n<const N>` for single bounds check (-10.4%)
2. ~~Optimized read_byte/read_bool/read_str/read_string with get_unchecked~~ → **ABLATED AWAY** (safe indexing is equally fast)
3. `#[inline]` on all generated decode methods (-4.4%)
4. `Vec::with_capacity` and `HashMap::with_capacity` instead of new (-2.7%)
5. **`simdutf8` for SIMD-accelerated UTF-8 validation** (-28.2% — biggest single win)
6. `#[inline]` on generated struct decode methods (downgraded from `#[inline(always)]` with no cost)
7. `#[inline(always)]` on generated enum decode methods (-1.5%)
8. **`hashbrown::HashMap` (foldhash) always** — breaking API change (-1.7% total, JsonLarge -17.7%)

## Ablation Results (Simplification Confirmed)
- **get_unchecked REMOVED**: All `unsafe { get_unchecked }` in read_byte/read_bool/read_str/read_string is unnecessary. LLVM eliminates bounds checks after manual `if pos < len` / `if end > len` checks. Zero performance cost to use safe indexing.
- **from_utf8_unchecked REMOVED**: simdutf8::basic::from_utf8() returns `&str` directly, no need for separate validate + unchecked. Eliminates 2 unsafe blocks.
- **ptr::read_unaligned REMOVED**: `buf[pos..pos+N].try_into().unwrap()` generates identical code to `ptr::read_unaligned` when N is const generic. LLVM optimizes perfectly.
- **#[inline(always)] → #[inline] on reader.rs**: All 22 `#[inline(always)]` downgraded to `#[inline]` with marginal improvement. Compiler makes better decisions without being forced.
- **#[inline(always)] → #[inline] on struct decode**: Struct decode methods work fine with `#[inline]`. Only enum decode needs `#[inline(always)]`.

## Confirmed Essential (Cannot Simplify)
- **simdutf8**: +46% regression without it. NEON SIMD gives 2x+ speedup for large strings.
- **Vec/HashMap::with_capacity**: +16% regression without it. Multiple reallocations are expensive.
- **#[inline(always)] on enum decode**: Enums are tiny (1 byte + match). Without forced inlining, call overhead dominates. +3.9% regression when removed.
- **#[inline] on message/union decode**: +5.4% regression without it. Cross-crate code won't inline tag loops without the hint.
- **hashbrown HashMap**: Foldhash is ~2x faster than SipHash for small keys.

## Remaining Bottlenecks (require structural changes)
- **Recursive Vec allocations** (TreeDeep ~1600ns): Each node allocates a Vec. SmallVec<[T;1]> would help but changes public API.
- **UTF-8 validation floor** (DocumentLarge ~2800ns, ChunkedText ~2000ns): simdutf8 is already near-optimal. ~146KB × ~13ns/KB = ~1900ns theoretical minimum.
- **Per-element overhead in arrays** (TreeWide ~880ns): 100 message decodes with tag loops. Fundamental overhead of the wire format.

## Future Ideas (require API or structural changes)
- **SmallVec for arrays in recursive types** (if API break acceptable): Estimate ~1000-1500ns reduction for TreeDeep. Feature-gated behind cargo feature flag.
- **Arena allocator for recursive types**: Custom allocator that batches small allocations. Significant engineering effort.
- **Lazy UTF-8 validation**: Validate only on `.as_str()` access. Major ergonomic regression — not recommended.
