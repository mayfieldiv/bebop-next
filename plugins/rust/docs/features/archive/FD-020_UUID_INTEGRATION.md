# FD-020: UUID Crate Integration

**Status:** Complete
**Completed:** 2026-02-20

## Summary

Added feature-gated `uuid` crate integration. UUID field type remains `[u8; 16]` (canonical byte order per whitepaper, not Microsoft swapped like production bebop). Users opt into `uuid::Uuid` interop via `.into()` conversions when the `uuid` Cargo feature is enabled. No code generation changes needed.

## Source

Migrated from `../../issues/archived/uuid-integration.md`
