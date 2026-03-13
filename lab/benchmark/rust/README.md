# Benchmark Comparison Tool

Runs Rust Bebop benchmarks and generates cross-language comparison reports.

The binary lives at `plugins/rust/benchmarks/src/bin/comparison.rs`.

## Prerequisites

- Rust toolchain
- `bin/bebopc` for type generation (`plugins/rust/benchmarks/generate.sh`)

## Usage

```bash
# Run Rust benchmarks only:
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --out lab/benchmark/results/rust_benchmark.json

# Run Rust benchmarks + compare against C:
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --out lab/benchmark/results/rust_benchmark.json \
  --lang c=lab/benchmark/results/c_benchmark.json \
  --baseline c

# Compare existing results (no benchmark run):
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --lang rust=lab/benchmark/results/rust_benchmark.json \
  --lang c=lab/benchmark/results/c_benchmark.json \
  --baseline c

# Three-language comparison:
cargo run --release -p bebop-rust-benchmarks --bin comparison -- \
  --lang c=c.json --lang rust=rust.json --lang swift=swift.json \
  --baseline c --report report.md
```

## Options

| Flag | Description |
|------|-------------|
| `--out <path>` | Run Rust benchmarks, write JSON (adds "rust" to comparison) |
| `--lang name=path` | Add a language's benchmark JSON (repeatable) |
| `--baseline name` | Baseline language for "vs" column (default: first language) |
| `--report <path>` | Write markdown report to file (default: stdout) |

All benchmark JSON files use Google Benchmark format (`BM_Bebop_{Encode\|Decode}_{Scenario}` naming).
