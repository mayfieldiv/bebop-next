# Rust Benchmark Harness

Runs Rust Bebop benchmarks and generates a Rust-vs-C comparison report.

The benchmark binary lives at `plugins/rust/benchmarks/src/bin/comparison.rs`.

## Prerequisites

- Rust toolchain
- `bin/bebopc` for type generation (`plugins/rust/benchmarks/generate.sh`)
- C benchmark JSON (either built from `lab/benchmark/c/` or use the checked-in snapshot)

## Usage

```bash
# Run benchmarks only:
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --out lab/benchmark/results/rust_benchmark.json

# Run benchmarks + generate comparison report:
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --out lab/benchmark/results/rust_benchmark.json \
  --c-json lab/benchmark/results/c_benchmark.json \
  --report lab/benchmark/results/benchmark_report.md

# Compare existing results (no benchmark run):
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --rust-json lab/benchmark/results/rust_benchmark.json \
  --c-json lab/benchmark/results/c_benchmark.json \
  --report lab/benchmark/results/benchmark_report.md
```

## Outputs

- `lab/benchmark/results/rust_benchmark.json` — Rust benchmark results (Google Benchmark JSON format)
- `lab/benchmark/results/benchmark_report.md` — Rust vs C comparison table
