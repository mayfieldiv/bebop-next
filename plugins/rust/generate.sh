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

echo "Generating descriptor.rs + plugin.rs..."
"$BEBOPC" build \
  "$SCHEMAS/bebop/descriptor.bop" \
  "$SCHEMAS/bebop/plugin.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q

mkdir -p "$RUST_DIR/src/generated"
cp "$TMPDIR/out/descriptor.rs" "$RUST_DIR/src/generated/descriptor.rs"
cp "$TMPDIR/out/plugin.rs" "$RUST_DIR/src/generated/plugin.rs"
rustfmt "$RUST_DIR/src/generated/descriptor.rs" "$RUST_DIR/src/generated/plugin.rs"

echo "Done."
