# Autoresearch Ideas — Rust Decode Performance

## Summary
Total improvement: **~47%** (20,062ns → ~10,700ns) over 67 experiments across 8 sessions.

## Optimizations Applied (Kept) — in order of impact
1. **`simdutf8` for SIMD-accelerated UTF-8 validation** (-28.2%)
2. `#[inline]` on all reader methods + `read_n<const N>` for bounded reads (-10.4%)
3. `#[inline]` on all generated decode methods (struct, message, union) (-4.4%)
4. `codegen-units = 1` + ThinLTO release profile (-4.2% combined)
5. `Vec::with_capacity` and `HashMap::with_capacity` (-2.7%)
6. **`hashbrown::HashMap` always** — breaking API change (JsonLarge -17.7%)
7. `read_array` ptr::write + set_len instead of push (TreeWide -13%)
8. `#[inline(always)]` on generated enum decode methods only (-1.5%)
9. Simplified `ensure()` using wrapping_sub (~marginal)

## Code Quality Improvements (zero perf cost)
- Removed ALL unsafe `get_unchecked` from hot paths
- Removed `from_utf8_unchecked` — simdutf8 returns `&str` directly
- Removed `ptr::read_unaligned` — safe `try_into().unwrap()` identical
- Downgraded `#[inline(always)]` → `#[inline]` on reader + struct decode

## Build Configuration
```toml
[profile.release]
codegen-units = 1  # -2.9%
lto = "thin"       # -1.3%
# panic = "abort"  # ~2% additional (user recommendation only — changes semantics)
```

## Plateau — Exhaustively Confirmed
67 experiments across 8 sessions. Every reader method, generated decode pattern, build flag,
inline annotation level, code generator pattern, and fused-read approach has been tested.

### Dead Ends Confirmed (with experiment numbers)
- #[cold] on error paths (runs 7, 56 — hurts code layout)
- Shrinking DecodeError (run 55 — borderline Result size → worse LLVM codegen)
- Pre-validating message bodies (runs 9, 54 — extra ensure, LLVM can't propagate)
- insert_unique_unchecked for HashMap (run 58 — probing rarely compares keys)
- Vec→collect for HashMap construction (run 59 — extra alloc)
- Fat LTO (run 51 — over-inlines recursive decode, TreeDeep +61%)
- FxHash hasher swap (run 60 — too invasive, foldhash already optimal)
- opt-level=2 (run 65 — worse than O3 for our patterns)
- Message decode loop→while (run 66 — removing position check hurts TreeWide +13%)
- Fused message header 5-byte read (run 67 — within noise, LLVM already merges checks)
- Various micro: buf.get() (41), pos!=len (57), read_n::<1>() (43), inlined u32 in read_str (46)
- Encoding-side changes don't affect decode metrics

### Unsafe Ablation Results (runs 31-35, 62-64)
- `get_unchecked` in read_byte/read_bool/read_str/read_string → REMOVED (safe indexing identical)
- `from_utf8_unchecked` → REMOVED (simdutf8 returns &str directly)
- `ptr::read_unaligned` in read_n → REMOVED (try_into identical)
- `read_array` ptr::write + set_len → ESSENTIAL (+3.8% regression without it)
- `read_scalar_array` from_raw_parts (aligned) → ESSENTIAL (zero-copy feature)
- `read_scalar_array` copy_nonoverlapping (unaligned) → ESSENTIAL (+4200% regression)

## Remaining Bottlenecks (immovable or structural)
- **UTF-8 validation floor** (~4,800ns): simdutf8 near-optimal on NEON
- **Recursive Vec allocs** (TreeDeep ~1,450ns): SmallVec<[T;1]> would help
- **HashMap operations** (JsonLarge ~1,500ns): foldhash already optimal
- **String array decode** (LLMChunkLarge ~1,250ns): UTF-8 + alloc dominated
- **Per-element message decode** (TreeWide ~650ns): wire format overhead

## Future Ideas (require API changes or major engineering)
- **SmallVec for recursive types**: ~800-1000ns potential for TreeDeep. Needs API approval.
- **Arena allocator**: Batch all decode allocations. Major engineering effort.
