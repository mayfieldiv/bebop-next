#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RUST_DIR="$REPO_ROOT/plugins/rust"

cd "$RUST_DIR"

echo "Checking formatting..."
cargo fmt --all -- --check

echo "Compiler checks..."
cargo check -p bebopc-gen-rust
cargo check -p bebop-runtime
cargo check -p bebop-runtime --no-default-features
cargo check -p bebop-integration-tests
cargo check -p bebop-integration-tests --no-default-features
cargo check -p bebop-rust-benchmarks --all-targets

echo "Generator tests..."
cargo test -p bebopc-gen-rust

echo "Runtime tests..."
cargo test -p bebop-runtime
cargo test -p bebop-runtime --no-default-features

echo "Integration tests..."
cargo test -p bebop-integration-tests
cargo test -p bebop-integration-tests --no-default-features

echo "Benchmark crate tests..."
cargo test -p bebop-rust-benchmarks

echo "Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "Golden file compatibility tests..."
C_DIR="$REPO_ROOT/lab/benchmark/c"
C_BUILD="$C_DIR/build"

# Generate C code from schema if needed
if [ ! -f "$C_DIR/generated/benchmark.bb.c" ]; then
  "$REPO_ROOT/bin/bebopc" build "$C_DIR/schemas/benchmark.bop" \
    -I "$REPO_ROOT/bebop/schemas" --c_out="$C_DIR/generated" -q
fi

mkdir -p "$C_BUILD"
if [ ! -f "$C_BUILD/CMakeCache.txt" ]; then
  cmake -S "$C_DIR" -B "$C_BUILD" -DCMAKE_BUILD_TYPE=Release -DBENCH_BEBOP=ON 2>&1 | tail -1
fi
cmake --build "$C_BUILD" --target dump_golden -j 2>&1 | tail -1

GOLDEN_DIR=$(mktemp -d)
cleanup() { rm -rf "$GOLDEN_DIR"; }
trap cleanup EXIT
# Run from lab/benchmark/c so relative fixture paths resolve correctly
(cd "$C_DIR" && "$C_BUILD/dump_golden" "$GOLDEN_DIR")

GOLDEN_DIR="$GOLDEN_DIR" cargo test -p bebop-rust-benchmarks --test golden

echo "All checks passed."
