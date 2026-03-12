# FD-011: Immutability Support

**Status:** Closed
**Priority:** N/A
**Effort:** N/A
**Impact:** N/A

## Problem

The Rust plugin ignores the `is_mutable` flag on `StructDef`. C uses `const` qualifiers; Swift uses `let` vs `var`. Should Rust enforce field-level immutability?

## Resolution: No Change Needed

Rust's ownership model handles mutability at the binding site (`let` vs `let mut`), not at the field level. Making fields private with getters would:

1. Not add real safety — Rust already prevents mutation without `let mut`
2. Reduce ergonomics — `point.x()` instead of `point.x`, no destructuring
3. Add complexity — getter return type logic, Copy vs reference handling
4. Diverge from Rust conventions — idiomatic Rust uses `pub` fields when no invariants to protect

The `is_mutable` distinction is meaningful for C and Swift where mutability is a property of the field declaration. In Rust, it's a property of the *binding*, making field-level enforcement redundant.

## Source

Migrated from `../../issues/plans/immutability.md`
