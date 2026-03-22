#!/usr/bin/env bash
# Autoresearch benchmark: runs the C/Rust comparison benchmark and extracts decode metrics
set -euo pipefail
cd "$(dirname "$0")"

# Run the comparison benchmark and capture JSON output
TMPJSON=$(mktemp /tmp/bench_rust_XXXXXX.json)
trap 'rm -f "$TMPJSON"' EXIT

cargo run --release -p bebop-rust-benchmarks --bin comparison -- --out "$TMPJSON" 2>/dev/null

# Parse the JSON to extract decode mean times
python3 -c "
import json, sys

with open('$TMPJSON') as f:
    data = json.load(f)

total_decode_ns = 0.0
scenarios = {}

for row in data['benchmarks']:
    if row.get('aggregate_name') != 'mean':
        continue
    name = row['run_name']
    if not name.startswith('BM_Bebop_'):
        continue
    rest = name[len('BM_Bebop_'):]
    parts = rest.split('_', 1)
    if len(parts) != 2:
        continue
    op, scenario = parts
    if op == 'Decode':
        ns = row['real_time']
        total_decode_ns += ns
        scenarios[scenario] = ns

print(f'METRIC total_decode_ns={total_decode_ns:.2f}')
for scenario, ns in sorted(scenarios.items()):
    print(f'METRIC decode_{scenario}_ns={ns:.2f}')
"
