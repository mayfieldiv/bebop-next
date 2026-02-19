#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RUST_DIR="$REPO_ROOT/plugins/rust"
INT_DIR="$RUST_DIR/integration-tests"
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

echo "Generating test_types.rs..."
"$BEBOPC" build \
  "$INT_DIR/schemas/test_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q 2>/dev/null

cp "$TMPDIR/out/test_types.rs" "$INT_DIR/src/test_types.rs"

echo "Generating no_std_types.rs..."
"$BEBOPC" build \
  "$INT_DIR/schemas/no_std_types.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q 2>/dev/null

cp "$TMPDIR/out/no_std_types.rs" "$INT_DIR/src/no_std_types.rs"

echo "Done."
