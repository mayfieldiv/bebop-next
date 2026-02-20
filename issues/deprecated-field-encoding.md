# Document Deprecated Field Encoding Behavior

- [x] Document deprecated message field encode/decode behavior divergence #rust-plugin 🔽

The C plugin skips deprecated message fields during encoding (they are decoded but never written). The Swift plugin emits `@available(*, deprecated)` but still encodes them.

## Current Rust Behavior
The Rust plugin emits `#[deprecated(note = "...")]` on the field but does NOT skip it during encode. It behaves like Swift.

## Action Taken
Added a generated-code note in `gen_message.rs` documenting:
- the current Rust behavior (encode/decode deprecated fields normally)
- C plugin behavior (skip deprecated fields during encode/size)
- Swift behavior (encode normally)
- that this should be revisited once spec intent is clarified

## Follow-up
Spec clarification is still needed to decide whether Rust should eventually skip deprecated fields on encode.

## References
- C: `generator.c` checks `is_deprecated` and skips field in `_Encode`
- Swift: always encodes, just adds `@available(*, deprecated)`
- Whitepaper: §Schema Evolution mentions deprecation but doesn't specify encode behavior
