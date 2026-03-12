# FD-012: Struct Layout Optimization

**Status:** Closed
**Priority:** N/A
**Effort:** N/A
**Impact:** N/A

## Problem

The C plugin reorders struct fields by alignment (descending) to minimize padding. Should the Rust plugin do the same?

## Resolution: No Change Needed

Rust's default struct layout (`repr(Rust)`) already allows the compiler to reorder fields for optimal padding. The compiler's field reordering is more sophisticated than any manual sort (it considers full struct layout holistically). Reordering in generated source would:

1. Not improve memory layout — compiler already does this
2. Hurt readability — fields would no longer match schema order
3. Not affect wire format — encode/decode use declaration order regardless

A spec-level `blittable struct` annotation enabling cross-language memory mapping is an interesting future idea, but outside the scope of the Rust plugin alone.

## Source

Migrated from `../../issues/plans/struct-layout-optimization.md`
