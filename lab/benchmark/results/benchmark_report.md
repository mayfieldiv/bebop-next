# Rust vs C Bebop Benchmarks

- Rust source: `/Users/mayfield/vendor/bebop-next/lab/benchmark/results/rust_benchmark.json`
- C source: `/Users/mayfield/vendor/bebop-next/lab/benchmark/results/c_benchmark.json`

| Scenario | Metric | Rust ns | C ns | Rust/C Speedup | Rust MiB/s | C MiB/s | Encoded Size (bytes) |
|---|---:|---:|---:|---:|---:|---:|---:|
| ChunkedText | encode | - | 4878.60 | - | - | 30028.71 | - |
| ChunkedText | decode | - | 3150.77 | - | - | 46497.29 | - |
| DocumentLarge | encode | 6125.45 | 79.35 | 0.01x | 22934.45 | 26417.43 | 147308 |
| DocumentLarge | decode | 6438.75 | 52.52 | 0.01x | 21818.50 | 39925.30 | 147308 |
| DocumentSmall | encode | 83.79 | 10.32 | 0.12x | 887.78 | 4806.27 | 78 |
| DocumentSmall | decode | 99.27 | 4.92 | 0.05x | 749.30 | 10085.82 | 78 |
| Embedding1536 | encode | - | 34.16 | - | - | 86336.44 | - |
| Embedding1536 | decode | - | 2.80 | - | - | 1054575.91 | - |
| Embedding384 | encode | - | 10.43 | - | - | 72064.06 | - |
| Embedding384 | decode | - | - | - | - | - | - |
| Embedding768 | encode | - | 18.92 | - | - | 78464.29 | - |
| Embedding768 | decode | - | 2.92 | - | - | 508707.72 | - |
| EmbeddingBatch | encode | - | 285.82 | - | - | 82667.11 | - |
| EmbeddingBatch | decode | - | 25.65 | - | - | 923358.62 | - |
| EmbeddingF32_768 | encode | - | 34.26 | - | - | 86100.87 | - |
| EmbeddingF32_768 | decode | - | - | - | - | - | - |
| EventLarge | encode | - | 49.43 | - | - | 80452.45 | - |
| EventLarge | decode | - | 6.00 | - | - | 664227.41 | - |
| EventSmall | encode | - | 10.67 | - | - | 3754.53 | - |
| EventSmall | decode | - | 5.84 | - | - | 6865.88 | - |
| InferenceResponse | encode | 2787.87 | 72.08 | 0.03x | 2172.89 | 82981.34 | 6352 |
| InferenceResponse | decode | 6085.44 | 17.53 | 0.00x | 995.45 | 341212.96 | 6352 |
| JsonLarge | encode | 3436.52 | 914.42 | 0.27x | 402.11 | 3153.88 | 1449 |
| JsonLarge | decode | 2863.99 | 1105.20 | 0.39x | 482.50 | 2611.78 | 1449 |
| JsonSmall | encode | - | 36.39 | - | - | 2568.04 | - |
| JsonSmall | decode | - | 40.60 | - | - | 2306.88 | - |
| LLMChunkLarge | encode | - | 1086.63 | - | - | 2771.23 | - |
| LLMChunkLarge | decode | - | 709.31 | - | - | 4244.59 | - |
| LLMChunkSmall | encode | - | 34.76 | - | - | 2935.54 | - |
| LLMChunkSmall | decode | - | - | - | - | - | - |
| OrderLarge | encode | - | 20.18 | - | - | 58589.66 | - |
| OrderLarge | decode | - | 5.83 | - | - | 202936.92 | - |
| OrderSmall | encode | - | 9.51 | - | - | 7629.95 | - |
| OrderSmall | decode | - | 5.86 | - | - | 12370.37 | - |
| PersonMedium | encode | 23.68 | 9.61 | 0.41x | 2940.42 | 12903.88 | 73 |
| PersonMedium | decode | 14.64 | 4.09 | 0.28x | 4756.07 | 30329.02 | 73 |
| PersonSmall | encode | 24.05 | 7.79 | 0.32x | 1585.87 | 3430.92 | 40 |
| PersonSmall | decode | 14.48 | 4.09 | 0.28x | 2633.87 | 6528.04 | 40 |
| TensorShardLarge | encode | 313063.69 | 561.60 | 0.00x | 2246.18 | 111443.16 | 737356 |
| TensorShardLarge | decode | 723649.71 | 7.19 | 0.00x | 971.74 | 8723976.10 | 737356 |
| TensorShardSmall | encode | - | 29.30 | - | - | 69407.34 | - |
| TensorShardSmall | decode | - | - | - | - | - | - |
| TreeDeep | encode | 3608.54 | 3584.35 | 0.99x | 257.68 | 3401.74 | 975 |
| TreeDeep | decode | 3380.39 | 5342.52 | 1.58x | 275.07 | 2287.20 | 975 |
| TreeWide | encode | - | 315.98 | - | - | 3064.01 | - |
| TreeWide | decode | - | 489.94 | - | - | 2019.99 | - |
