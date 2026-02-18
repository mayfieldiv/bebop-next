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

# Strip the generator preamble from a .bb.rs file.
# Removes leading comments (//), inner attributes (#![), use statements, and blank lines
# until we hit the first doc comment (///) or derive attribute (#[derive).
strip_preamble() {
  awk '
    BEGIN { p = 0 }
    !p && /^(\/\/[^\/]|\/\/$|#!\[|use |$)/ { next }
    { p = 1; print }
  ' "$1"
}

echo "Generating descriptor.bb.rs + plugin.bb.rs..."
"$BEBOPC" build \
  "$SCHEMAS/bebop/descriptor.bop" \
  "$SCHEMAS/bebop/plugin.bop" \
  -I "$SCHEMAS" \
  --plugin=rust="$PLUGIN" \
  --rust_out="$TMPDIR/out" \
  -q

mkdir -p "$RUST_DIR/src/generated"
strip_preamble "$TMPDIR/out/descriptor.bb.rs" > "$RUST_DIR/src/generated/descriptor.bb.rs"
strip_preamble "$TMPDIR/out/plugin.bb.rs" > "$RUST_DIR/src/generated/plugin.bb.rs"

echo "Done."
