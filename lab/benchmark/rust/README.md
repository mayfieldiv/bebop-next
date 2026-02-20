# Rust Benchmark Harness

Runs Rust Bebop benchmarks and generates a Rust-vs-C comparison report.

## Prerequisites

- Rust toolchain
- `bin/bebopc` available for type generation (`plugins/rust/benchmarks/generate.sh`)
- Optional C benchmark build at `lab/benchmark/c/build/run_benchmarks`

## Run

```bash
lab/benchmark/rust/run_comparison.sh
```

Outputs:

- `lab/benchmark/results/rust_benchmark.json`
- `lab/benchmark/results/c_benchmark.json`
- `lab/benchmark/results/benchmark_report.md`
