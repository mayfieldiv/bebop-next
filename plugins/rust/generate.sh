#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUST_DIR="$REPO_ROOT/plugins/rust"
SCHEMAS="$REPO_ROOT/bebop/schemas"
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

# ── 1. Bootstrap: descriptor.rs + plugin.rs ──────────────────────────

echo "Generating descriptor.rs + plugin.rs..."
"$BEBOPC" build \
  "$SCHEMAS/bebop/descriptor.bop" \
  "$SCHEMAS/bebop/plugin.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/bootstrap" \
  -q

mkdir -p "$RUST_DIR/src/generated"
cp "$TMPDIR/bootstrap/descriptor.rs" "$RUST_DIR/src/generated/descriptor.rs"
cp "$TMPDIR/bootstrap/plugin.rs" "$RUST_DIR/src/generated/plugin.rs"

# ── 2. Integration tests: test_types.rs ──────────────────────────────

echo "Generating test_types.rs..."
"$BEBOPC" build \
  "$RUST_DIR/integration-tests/schemas/test_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/integration" \
  --rust_opt=serde \
  -q 2>/dev/null

cp "$TMPDIR/integration/test_types.rs" "$RUST_DIR/integration-tests/src/test_types.rs"

echo "Generating collision_types.rs..."
"$BEBOPC" build \
  "$RUST_DIR/integration-tests/schemas/collision_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/collision" \
  --rust_opt=serde \
  -q 2>/dev/null

cp "$TMPDIR/collision/collision_types.rs" "$RUST_DIR/integration-tests/src/collision_types.rs"

# ── 3. Benchmarks: benchmark_types.rs ────────────────────────────────

echo "Generating benchmark_types.rs..."
"$BEBOPC" build \
  "$REPO_ROOT/lab/benchmark/c/schemas/benchmark.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/bench" \
  -q 2>/dev/null

cp "$TMPDIR/bench/benchmark.rs" "$RUST_DIR/benchmarks/src/benchmark_types.rs"

echo "Done. Regenerated all generated files."
