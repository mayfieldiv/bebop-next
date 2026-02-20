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

# Work around a known name collision when benchmark.bop defines `message String`
# inside `union JsonValue`. The generated file imports alloc::string::String,
# which clashes with the generated `String` message type.
perl -0pi -e 's/use alloc::string::String;/use alloc::string::String as StdString;/g' \
  "$BENCH_DIR/src/benchmark_types.rs"
perl -0pi -e 's/pub fn new\\(chunk_id: u32, tokens: Vec<String>, logprobs: Vec<TokenAlternatives<\\x27static>>, finish_reason: impl Into<Cow<\\x27buf, str>>\\) -> Self \\{\\n    let finish_reason = finish_reason.into\\(\\);\\n    Self \\{ chunk_id, tokens, logprobs, finish_reason \\}/pub fn new(chunk_id: u32, tokens: Vec<StdString>, logprobs: Vec<TokenAlternatives<\\x27static>>, finish_reason: impl Into<Cow<\\x27buf, str>>) -> Self {\\n    let tokens = tokens.into_iter().map(Cow::Owned).collect();\\n    let finish_reason = finish_reason.into();\\n    Self { chunk_id, tokens, logprobs, finish_reason }/s' \
  "$BENCH_DIR/src/benchmark_types.rs"
perl -0pi -e 's/#\\[derive\\(Debug, Clone, Default, PartialEq, Eq, Hash\\)\\]\\npub struct List<\\x27buf>/#[derive(Debug, Clone, Default, PartialEq)]\\npub struct List<\\x27buf>/g' \
  "$BENCH_DIR/src/benchmark_types.rs"

echo "Done."
