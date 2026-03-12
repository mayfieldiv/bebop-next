# FD-016: Generator Dogfooding

**Status:** Complete
**Completed:** 2026-02-20

## Summary

Eliminated benchmark post-generation patching by fixing generator output at the source:

- Replaced string type collision alias (`StdString`) with fully qualified paths (`alloc::...`, `::core::...`, `::bebop_runtime::...`)
- Emitted `#![no_implicit_prelude]` with explicit core prelude import
- Fixed constructor codegen for lifetime-bearing containers (`Vec<String>` -> `Vec<Cow<'buf, str>>`)
- Removed all Perl patching from `benchmarks/generate.sh`
- Added `rustfmt` execution in generation scripts

## Source

Migrated from `../../issues/archived/generator-dogfooding.md`
