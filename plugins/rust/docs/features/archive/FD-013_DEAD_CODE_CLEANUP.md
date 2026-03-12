# FD-013: Dead Code Cleanup

**Status:** Complete
**Completed:** 2026-02-19

## Summary

Removed three dead `#[allow(dead_code)]` functions in `type_mapper.rs` that were kept during the Cow migration refactor: `rust_type()`, `read_expression()`, `write_expression()`. The codebase had fully migrated to `_cow` variants.

## Source

Migrated from `../../issues/archived/dead-code-cleanup.md`
