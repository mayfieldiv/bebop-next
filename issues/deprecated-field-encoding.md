# Verify Deprecated Field Encoding Behavior

- [ ] Verify deprecated message fields are skipped during encode #rust-plugin 🔽

The C plugin skips deprecated message fields during encoding (they are decoded but never written). The Swift plugin emits `@available(*, deprecated)` but still encodes them.

## Question
What is the bebop-next intended behavior for deprecated fields?
- **Skip on encode** (C behavior): deprecated fields are never serialized, only deserialized for backward compat
- **Warn but encode** (Swift behavior): deprecated fields are still fully functional, just annotated

## Current Rust Behavior
The Rust plugin emits `#[deprecated(note = "...")]` on the field but does NOT skip it during encode. It behaves like Swift.

## Action
Verify which behavior the whitepaper/spec intends. If skip-on-encode is correct, update `gen_message.rs` to wrap deprecated fields' encode logic in a comment/skip. If warn-only is correct, document this as intentional divergence from C.

## References
- C: `generator.c` checks `is_deprecated` and skips field in `_Encode`
- Swift: always encodes, just adds `@available(*, deprecated)`
- Whitepaper: §Schema Evolution mentions deprecation but doesn't specify encode behavior
