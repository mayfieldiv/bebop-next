# FD-025: Forward-Compatible Message Field Ordering Semantics

**Status:** Design
**Priority:** Medium
**Effort:** Low (documentation + possible test)
**Impact:** Prevents subtle data loss when evolving forward-compatible messages

## Problem

When a forward-compatible message encounters an unknown tag during decode, the generated code skips ALL remaining bytes to the end of the message body:

```rust
// test_types.rs — FlexConfig decode
_ => {
    reader.skip(end - reader.position())?;
}
```

This means if a newer schema adds field tag 3 and the old decoder encounters it, **all subsequent fields (including known ones at higher tags) are silently lost**. This is a protocol-level constraint — Bebop messages don't length-prefix individual fields, so the decoder can't skip just one unknown field.

### Surprising Consequence

Field ordering in the **encoder** matters for forward compatibility. If a newer encoder writes fields in tag order (1, 2, 3, 4) and an older decoder only knows tags 1, 2, and 4, hitting unknown tag 3 causes tag 4 to be skipped. The data round-trips with tag 4 silently dropped.

This is not obvious to users and contradicts the common assumption that tagged message fields can be read in any order.

## Investigation Needed

1. **Review the Bebop wire spec** — Confirm this is intentional behavior. Check the whitepaper and other language implementations (C, TypeScript, Swift) to see if they handle unknown tags the same way.
2. **Check encoding order** — Does the Bebop compiler guarantee fields are encoded in tag order? If so, the "known fields after unknown" scenario only happens when new tags are added between existing ones (which would be a schema evolution mistake anyway).
3. **Check other implementations** — Do C/TypeScript/Swift generators emit the same skip-to-end pattern?
4. **Determine if Bebop messages always terminate with tag 0** — If so, the skip-to-end is safe as long as new fields are appended (higher tags than existing).

## Possible Outcomes

- **If by design:** Document the constraint clearly. Add a doc comment to the generated forward-compatible decode noting that unknown tags skip remaining fields. Possibly add a note in `research.md`.
- **If a bug:** The fix would need to be at the wire format level (per-field length prefixing), which is a cross-language breaking change — likely not feasible.

## Files to Investigate

| File | Purpose |
|------|---------|
| `integration-tests/src/test_types.rs:1130-1148` | FlexConfig forward-compatible decode |
| `src/generator/gen_message.rs` | Unknown tag skip generation |
| Bebop whitepaper / wire spec | Canonical message encoding rules |
| C/TypeScript/Swift generators | Cross-language comparison |

## Verification

- Construct a test case with a forward-compatible message where an unknown tag appears before a known tag
- Verify behavior matches other language implementations
- Document findings regardless of outcome
