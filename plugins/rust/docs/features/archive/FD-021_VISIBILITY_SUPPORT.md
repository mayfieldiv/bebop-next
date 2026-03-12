# FD-021: Visibility Support

**Status:** Complete
**Completed:** 2026-02-20

## Summary

Implemented `pub` vs `pub(crate)` visibility support based on the `Visibility` field in `DefinitionDescriptor`:

| Bebop Visibility | Rust Output |
|---|---|
| `Export` / `Default` (top-level) | `pub` |
| `Local` | `pub(crate)` |

Applied in all generators: `gen_struct`, `gen_message`, `gen_enum`, `gen_union`.

## Source

Migrated from `../../issues/archived/visibility-support.md`
