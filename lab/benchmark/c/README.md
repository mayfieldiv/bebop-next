# C Serialization Benchmarks

Compares Bebop's C runtime against Protobuf-C and MessagePack using [Google Benchmark](https://github.com/google/benchmark).

## Build

```bash
cd lab/benchmark/c
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release -DBENCH_PROTOBUF=ON -DBENCH_MSGPACK=ON
cmake --build . -j
```

Prerequisites:
- Protobuf-C: `brew install protobuf-c` (macOS) or `apt install libprotobuf-c-dev protobuf-compiler` (Linux)
- MessagePack: Fetched automatically via CMake

## Run

```bash
./run_benchmarks                          # All benchmarks
./run_benchmarks --benchmark_filter="Bebop.*"  # Filter by library
make bench                                # Console output
make bench-json                           # Save to results/benchmark.json
```

## Test Data

| Type | Description |
|------|-------------|
| Person | Simple struct with strings and integers |
| Order | Nested struct with arrays (variable-length encoding test) |
| Event | Binary payload (4 bytes small, 4KB large) |
| Tree | Recursive structure (wide: 100 children; deep: 100 levels) |
| JsonValue | Union type representing JSON AST |
| Document | Struct with map fields |
| ChunkedText | Semantic text spans (Alice in Wonderland, 8000+ chunks) |
