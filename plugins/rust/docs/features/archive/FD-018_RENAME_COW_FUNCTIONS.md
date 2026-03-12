# FD-018: Rename _cow Functions

**Status:** Complete
**Completed:** 2026-02-19

## Summary

After dead code removal (FD-013), renamed `_cow` suffixed functions to their canonical names:

- `rust_type_cow()` -> `rust_type()`
- `read_expression_cow()` -> `read_expression()`
- `write_expression_cow()` -> `write_expression()`

Updated all call sites in `gen_struct.rs` and `gen_message.rs`.

## Source

Migrated from `../../issues/archived/rename-cow-functions.md`
