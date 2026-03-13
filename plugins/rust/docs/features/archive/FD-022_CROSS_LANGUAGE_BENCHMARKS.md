# FD-022: Cross-Language Benchmark Expansion

**Status:** Complete
**Completed:** 2026-03-13
**Priority:** Medium
**Effort:** Medium (1-4 hours)
**Impact:** Full apples-to-apples Rust vs C comparison across all 23 scenarios; identifies performance regressions and optimization targets

## Problem

The cross-language benchmark infrastructure (`lab/benchmark/`) defines 23 scenarios across 5 categories, but the Rust benchmark harness only covers 8 of them. The report shows `-` for 15 Rust scenarios.

## Solution

### Phase 1: Rust benchmark runner ✅ DONE

- Replaced `compare.py` and `run_comparison.sh` with Rust binary
- N-language comparison with `--lang name=path` and `--baseline`
- Zero-arg default mode: runs benchmarks, auto-loads C snapshot
- `bench!` macro for one-liner scenario registration

### Phase 2: Complete Rust benchmark coverage

Add the 15 missing scenarios. Each needs a fixture in `lib.rs` and a `bench!()` call in `comparison.rs`.

The fixture data does NOT need to match C exactly — different languages may use different data. What matters is that scenario names match and the data is representative of the workload category (small struct, large array, recursive tree, etc.).

#### Missing scenarios

| Scenario | Type | Fixture spec | Iterations |
|----------|------|-------------|------------|
| **OrderSmall** | `Order` | 3 item_ids, 3 quantities, total, timestamp | 50_000 |
| **OrderLarge** | `Order` | 100 item_ids, 100 quantities | 10_000 |
| **EventSmall** | `Event` | short type/source strings, 4-byte payload | 50_000 |
| **EventLarge** | `Event` | longer strings, 4096-byte payload | 10_000 |
| **TreeWide** | `TreeNode` | 1 root with 100 leaf children | 5_000 |
| **JsonSmall** | `JsonValue` | small Object with 3 fields (String, Number, Bool) | 50_000 |
| **ChunkedText** | `ChunkedText` | Alice text with ~50 paragraph spans | 2_500 |
| **Embedding384** | `EmbeddingBf16` | UUID + 384 bf16 values | 20_000 |
| **Embedding768** | `EmbeddingBf16` | UUID + 768 bf16 values | 10_000 |
| **EmbeddingF32_768** | `EmbeddingF32` | UUID + 768 f32 values | 10_000 |
| **EmbeddingBatch** | `EmbeddingBatch` | model string, 8 embeddings (1536 bf16 each), usage_tokens | 2_000 |
| **LLMChunkSmall** | `LlmStreamChunk` | chunk_id, 3 tokens, 3 logprob entries (1 alt each) | 50_000 |
| **LLMChunkLarge** | `LlmStreamChunk` | chunk_id, 32 tokens, 32 logprob entries (5 alts each), finish_reason | 2_000 |
| **TensorShardSmall** | `TensorShard` | shape=[4096,4096], 1024 bf16 data elements | 10_000 |
| **InferenceResponse** | — | Already exists, keep current fixture | — |

Note: the existing `InferenceResponse` fixture is fine as-is. `Embedding1536` scenario name in C maps to `EmbeddingBf16` type — use `embedding_bf16(n)` parameterized fixture for 384/768/1536.

#### Existing fixtures to refactor

The existing `embedding_bf16_1536()` becomes `embedding_bf16(1536)` to support multiple sizes. Similarly `tree_deep(depth)` stays as-is, and we add `tree_wide(children)`.

### Phase 3: Criterion benchmarks

After comparison.rs is complete with all 23 scenarios, optionally add matching criterion bench groups for Rust-only regression tracking.

## Files to Modify

| File | Action | Purpose |
|------|--------|---------|
| `benchmarks/src/lib.rs` | MODIFY | Add 13 new fixture functions, parameterize embedding_bf16 |
| `benchmarks/src/bin/comparison.rs` | MODIFY | Add 15 `bench!()` calls for missing scenarios |

## Verification

1. `cargo run --release --bin comparison` shows all 23 scenarios with Rust numbers (no `-` in Rust column)
2. Scenario names in Rust JSON match C JSON exactly (join produces full table)
3. `cargo clippy -- -D warnings` passes
4. `cargo test -p bebop-rust-benchmarks` passes

## Related

- FD-010: Zero-Copy Bulk Arrays — would dramatically improve TensorShard/Embedding benchmarks
- FD-008: OOM Handling — allocation performance relevant to large payload benchmarks
- `lab/benchmark/c/schemas/benchmark.bop` — shared schema (source of truth for scenarios)
- `lab/benchmark/c/src/harness.cpp` — C fixture construction (reference for data shapes)
