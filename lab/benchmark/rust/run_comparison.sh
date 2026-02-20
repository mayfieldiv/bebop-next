#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RESULTS_DIR="$REPO_ROOT/lab/benchmark/results"
RUST_JSON="$RESULTS_DIR/rust_benchmark.json"
C_JSON="$RESULTS_DIR/c_benchmark.json"
REPORT_MD="$RESULTS_DIR/benchmark_report.md"

mkdir -p "$RESULTS_DIR"

echo "Running Rust benchmark harness..."
cargo run --release \
  --manifest-path "$REPO_ROOT/plugins/rust/benchmarks/Cargo.toml" \
  --bin comparison -- --out "$RUST_JSON"

if [ -x "$REPO_ROOT/lab/benchmark/c/build/run_benchmarks" ]; then
  echo "Running C benchmark harness..."
  "$REPO_ROOT/lab/benchmark/c/build/run_benchmarks" \
    --benchmark_format=json \
    --benchmark_out="$C_JSON" >/dev/null
else
  echo "C benchmark binary not found; using checked-in snapshot benchmark_results_st_3.json"
  cp "$REPO_ROOT/lab/benchmark/c/benchmark_results_st_3.json" "$C_JSON"
fi

echo "Generating comparison report..."
python3 "$REPO_ROOT/lab/benchmark/rust/compare.py" \
  --c-json "$C_JSON" \
  --rust-json "$RUST_JSON" \
  --out "$REPORT_MD"

echo "Wrote:"
echo "  $RUST_JSON"
echo "  $C_JSON"
echo "  $REPORT_MD"
