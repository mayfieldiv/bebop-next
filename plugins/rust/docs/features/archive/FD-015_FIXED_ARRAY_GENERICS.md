# FD-015: Fixed Array Generics

**Status:** Complete
**Completed:** 2026-02-19

## Summary

Generalized fixed array support beyond `i32`. Replaced `read_fixed_i32_array`/`write_fixed_i32_array` with generic `read_fixed_array<T: FixedScalar, const N: usize>` and `write_fixed_array<T: FixedScalar, const N: usize>` that work with all scalar types.

## Source

Migrated from `../../issues/archived/fixed-array-generics.md`
