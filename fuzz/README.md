# Bebop Fuzzing

Coverage-guided fuzzing for the Bebop parser using AFL++.

## Setup

### macOS
```bash
brew install aflplusplus
```

### Linux
```bash
apt install afl++
```

## Building

```bash
cd fuzz
make
```

This builds fuzz targets with AFL++ instrumentation:
- `fuzz_parse` - Full parser (scanner + parser + validator)
- `fuzz_scan` - Scanner only (lexical analysis)

## Running

### Interactive fuzzing

```bash
# Parser fuzzing (recommended - most coverage)
make run-parse

# Scanner-only fuzzing
make run-scan
```

### Parallel fuzzing

Run multiple instances for faster coverage:

```bash
# Terminal 1 - Main instance
make run-parse-main

# Terminal 2+ - Secondary instances
make run-parse-sec
```

## Crash Analysis

Build with AddressSanitizer to analyze crashes:

```bash
make asan

# Analyze a crash
./build/fuzz_parse_asan crash_file
```

## Corpus

Uses `tests/fixtures/valid/` as the seed corpus (shared with unit tests).
AFL++ mutates these to discover new code paths.

## Fuzz Targets

| Target | Description | Coverage |
|--------|-------------|----------|
| `fuzz_parse` | Full parsing pipeline | Scanner, parser, semantic analysis, validation |
| `fuzz_scan` | Lexical analysis only | Token scanning, string interning |

## Tips

1. **Start with `fuzz_parse`** - it covers the most code
2. **Check `build/out/*/crashes/`** for crash inputs
3. **Use ASAN builds** to get stack traces for crashes
4. **Add interesting inputs** to corpus when found manually
5. **Run for hours/days** for thorough coverage

## Output

AFL++ creates output in `build/out/<target>/`:
- `crashes/` - Inputs that caused crashes
- `hangs/` - Inputs that caused timeouts
- `queue/` - Interesting inputs discovered during fuzzing
