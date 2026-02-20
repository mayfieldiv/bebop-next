# Dogfood Generated Rust Output

- [x] Remove benchmark post-generation patching by fixing generator output at the source #rust-plugin ✅ 2026-02-20

`plugins/rust/benchmarks/generate.sh` previously applied Perl patches after generation to fix:
- `String` name collisions
- `LLMStreamChunk.tokens` constructor type mismatch
- incorrect derives for recursive JSON benchmark types

## Implemented
- Initially moved string type collision handling into generator imports/types (`StdString` alias).
- Follow-up (2026-02-20): replaced alias-based handling with qualified type paths (`alloc::...`, `::core::...`, `::bebop_runtime::...`) and removed colliding preamble imports so schemas can safely define names like `String`.
- Fixed constructor codegen to convert owned inputs into borrowed field forms for lifetime-bearing containers (for example `Vec<String>` -> `Vec<Cow<'buf, str>>`).
- Added regression tests in `plugins/rust/src/generator/mod.rs` for:
  - recursive union float propagation into nested message derive decisions
  - struct constructor conversion for string arrays
- Removed benchmark-specific post-copy patching from `plugins/rust/benchmarks/generate.sh`.
- Added `rustfmt` execution in Rust generation scripts so regenerated files satisfy `test.sh` formatting checks.

## Validation
- Regenerated all Rust outputs:
  - `plugins/rust/generate.sh`
  - `plugins/rust/integration-tests/generate.sh`
  - `plugins/rust/benchmarks/generate.sh`
- Ran `plugins/rust/test.sh` successfully (format, check, tests, clippy).
