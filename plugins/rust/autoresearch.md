# Autoresearch: Rust Decode Performance

## Goal
Improve Rust decode performance relative to C, measured by total decode nanoseconds across all comparison benchmark scenarios.

## Rules
- Only modify files in `runtime/src/` (the runtime library) — never modify benchmark types, fixtures, or the comparison binary
- Changes must pass `./test.sh` (full validation including golden files)
- Do NOT skip UTF-8 validation or safety checks
- Do NOT change the wire format
- Public API changes are allowed when explicitly approved (e.g., hashbrown HashMap)
- Ergonomics must not degrade (borrowing, Cow patterns, error handling)
- No benchmark-specific optimizations (must be general improvements)
- The code generator (`src/`) may be modified if it produces better decode code, but generated files must be regenerated and golden tests must pass

## Key Files
- `runtime/src/reader.rs` — BebopReader (core decode path)
- `runtime/src/traits.rs` — BebopDecode trait, FixedScalar, BulkScalar
- `benchmarks/src/benchmark_types.rs` — generated code (read-only for understanding patterns)
- `benchmarks/src/bin/comparison.rs` — comparison benchmark runner (read-only)

## Metrics
- Primary: `total_decode_ns` — sum of all decode scenario mean times
- Secondary: per-scenario decode times for tradeoff monitoring
