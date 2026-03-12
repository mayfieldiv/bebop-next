# FD-002: Deprecated Field Handling

**Status:** Planned
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Spec compliance for deprecated message fields

## Problem

The Bebop spec says deprecated message fields should be skipped during encoding and decoding. The current Rust plugin encodes them normally (matching the Swift plugin). The C plugin skips them on encode but still decodes them. Behavior needs to be clarified and aligned with the spec.

## Solution

Once the spec intent is clarified:
- If skip-on-encode: modify message encode to skip deprecated fields, adjust `encoded_size()` accordingly
- If skip-on-decode: modify message decode to ignore deprecated field tags
- May need both (C plugin approach: skip encode, keep decode for backwards compat)

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_message.rs` | MODIFY | Update encode/decode for deprecated fields |

## Verification

- Create test schema with deprecated message fields
- Verify encoding skips deprecated fields (if that's the spec intent)
- Verify decoding still handles deprecated fields for backward compat
- Cross-test with C/Swift plugin output

## Current State

Documentation of the behavior divergence has been added to `gen_message.rs`. The Rust plugin currently encodes deprecated fields normally (matching Swift). Blocked on spec clarification of intended behavior.

## Related

- `src/generator/gen_message.rs:203-215` — current NOTE comment
- `research.md` §4.6.3, §12.2
- C: skips deprecated fields during encode/size
- Swift: always encodes, just adds `@available(*, deprecated)`
- Whitepaper §Schema Evolution: mentions deprecation but doesn't specify encode behavior

## Source

Migrated from `../../issues/deprecated-field-encoding.md`
