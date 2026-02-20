# Rust Criterion Benchmarks

This crate benchmarks generated Rust Bebop types from `lab/benchmark/c/schemas/benchmark.bop`.

## Regenerate types

```bash
plugins/rust/benchmarks/generate.sh
```

## Run criterion benchmarks

```bash
cargo bench --manifest-path plugins/rust/benchmarks/Cargo.toml
```

## Run JSON comparison harness

```bash
cargo run --release --manifest-path plugins/rust/benchmarks/Cargo.toml --bin comparison -- --out lab/benchmark/results/rust_benchmark.json
```
