# FD-017: Insertion Point Markers

**Status:** Complete
**Completed:** 2026-02-19

## Summary

Added `// @@bebop_insertion_point(...)` markers throughout generated code, matching the C and Swift plugins. Markers allow downstream plugins to inject code at well-known locations.

Implemented markers:
- **File-level:** `imports`, `eof`
- **Type scope:** `struct_scope`, `message_scope`, `enum_scope`, `union_scope`
- **Encode/Decode:** `encode_start`, `encode_end`, `decode_start`, `decode_end`
- **Union-specific:** `encode_switch`, `decode_switch`

## Source

Migrated from `../../issues/archived/insertion-points.md`
