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

echo "Generator tests..."
cargo test -p bebopc-gen-rust

echo "Runtime tests..."
cargo test -p bebop-runtime
cargo test -p bebop-runtime --no-default-features

echo "Integration tests..."
cargo test -p bebop-integration-tests
cargo test -p bebop-integration-tests --no-default-features

echo "Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "All checks passed."
