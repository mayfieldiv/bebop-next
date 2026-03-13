# FD-022: Cross-Language Benchmark Expansion

**Status:** In Progress
**Priority:** Medium
**Effort:** Medium (1-4 hours)
**Impact:** Full apples-to-apples Rust vs C comparison across all 23 scenarios; identifies performance regressions and optimization targets

## Problem

The cross-language benchmark infrastructure (`lab/benchmark/`) defines 23 scenarios across 5 categories, but the Rust benchmark harness only covers ~10 of them. The benchmark report shows `-` for 13 Rust scenarios, making it impossible to compare languages on those workloads.

The comparison tooling is split across three languages:
- `comparison.rs` — runs Rust benchmarks, outputs JSON
- `run_comparison.sh` — orchestrates: runs Rust binary, runs/copies C results, calls compare.py
- `compare.py` — reads two JSON files, generates markdown report

This should all be one Rust program.

## Solution

### Phase 1: Rust benchmark runner (replaces shell + Python)

Extend `comparison.rs` to handle the full workflow:
- Run Rust benchmarks and write JSON (existing)
- Read C benchmark JSON (`--c-json <path>`)
- Generate markdown comparison report (`--report <path>`)

CLI:
```
# Run benchmarks only:
cargo run --release --bin comparison -- --out rust.json

# Run benchmarks + generate comparison report:
cargo run --release --bin comparison -- --out rust.json --c-json c.json --report report.md

# Compare existing results (no benchmark run):
cargo run --release --bin comparison -- --rust-json rust.json --c-json c.json --report report.md
```

Port the `compare.py` logic:
- Parse Google Benchmark JSON format (filter `aggregate_name == "mean"`)
- Parse `run_name` pattern: `BM_Bebop_{Encode|Decode}_{Scenario}`
- Join Rust + C results by scenario name
- Generate markdown table with ns, speedup, MiB/s, encoded size

### Phase 2: Complete Rust benchmark coverage

Add the 13 missing scenarios to the comparison binary and criterion benchmarks.

### Phase 3: Identify optimization targets (informational)

With full coverage, triage the performance gaps.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `benchmarks/src/bin/comparison.rs` | MODIFY | Add report generation, C JSON parsing, CLI args |
| `benchmarks/Cargo.toml` | MODIFY | No new deps needed (already has serde_json) |
| `lab/benchmark/rust/run_comparison.sh` | DELETE | Replaced by comparison binary |
| `lab/benchmark/rust/compare.py` | DELETE | Replaced by comparison binary |

## Verification

1. `cargo run --release --bin comparison -- --out /tmp/r.json --c-json <c-snapshot> --report /tmp/report.md` produces identical report to current `compare.py` output
2. `cargo check` passes with no warnings

## Related

- FD-010: Zero-Copy Bulk Arrays — would dramatically improve TensorShard/Embedding benchmarks
- FD-008: OOM Handling — allocation performance relevant to large payload benchmarks
- `lab/benchmark/c/schemas/benchmark.bop` — shared schema (source of truth for scenarios)
