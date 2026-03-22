#!/usr/bin/env bash
# Correctness checks: run the full test suite
set -euo pipefail
cd "$(dirname "$0")"
./test.sh
