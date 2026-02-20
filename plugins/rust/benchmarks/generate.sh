#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RUST_DIR="$REPO_ROOT/plugins/rust"
BENCH_DIR="$RUST_DIR/benchmarks"
BEBOPC="$REPO_ROOT/bin/bebopc"

if [ ! -x "$BEBOPC" ]; then
  echo "error: bebopc not found at $BEBOPC"
  echo "       install it to bin/ first"
  exit 1
fi

echo "Building bebopc-gen-rust..."
cargo build --manifest-path "$RUST_DIR/Cargo.toml" -q
PLUGIN="$RUST_DIR/target/debug/bebopc-gen-rust"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

SCHEMAS="$REPO_ROOT/bebop/schemas"
SRC_SCHEMA="$REPO_ROOT/lab/benchmark/c/schemas/benchmark.bop"

echo "Generating benchmark_types.rs..."
"$BEBOPC" build \
  "$SRC_SCHEMA" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q 2>/dev/null

cp "$TMPDIR/out/benchmark.rs" "$BENCH_DIR/src/benchmark_types.rs"
rustfmt "$BENCH_DIR/src/benchmark_types.rs"

echo "Done."
