# FD-029: Display for Enums and Flags

**Status:** Planned
**Priority:** Medium
**Effort:** Low (< 1 hour)
**Impact:** Enables human-readable logging and error messages for enum/flag values

## Problem

No generated type implements `Display`. For enums and flags especially, this is a missed opportunity:

```rust
// Current: only Debug is available
println!("{:?}", Color::Red);     // "Red" (Debug, works but not guaranteed stable)
println!("{:?}", perms);          // "Permissions(3)" (opaque)

// With Display:
println!("{}", Color::Red);       // "Red"
println!("{}", perms);            // "READ | WRITE"
```

`Debug` output is not contractually stable and can change between compiler versions. `Display` is the idiomatic way to provide human-readable string representations.

## Solution

### Enums

Generate `Display` that outputs the variant name as a string:

```rust
impl core::fmt::Display for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Color::Red => f.write_str("Red"),
            Color::Green => f.write_str("Green"),
            Color::Blue => f.write_str("Blue"),
        }
    }
}
```

For forward-compatible enums, the `Unknown(T)` variant displays as `"Unknown(<value>)"`.

### Flags

Generate `Display` that outputs pipe-separated known flag names:

```rust
impl core::fmt::Display for Permissions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut first = true;
        if self.contains(Permissions::READ) {
            f.write_str("READ")?;
            first = false;
        }
        if self.contains(Permissions::WRITE) {
            if !first { f.write_str(" | ")?; }
            f.write_str("WRITE")?;
            first = false;
        }
        // ...
        if first {
            // No known flags set
            write!(f, "0x{:x}", self.0)?;
        }
        Ok(())
    }
}
```

This mirrors the `bitflags` crate's Display behavior.

### What NOT to implement Display for

- **Structs/messages** — No single canonical human-readable representation. `Debug` suffices.
- **Unions** — Variant name could work but the payload complicates things. Defer.
- **Consts** — Not types, just values.

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/generator/gen_enum.rs` | MODIFY | Generate `Display` impl for enums and flags |
| `integration-tests/tests/integration.rs` | MODIFY | Test Display output |

## Verification

- `Display` output for each enum variant matches the variant name
- `Display` output for flags shows pipe-separated names
- Forward-compatible enum `Unknown` variant displays correctly
- Empty flags display as hex value
- `./test.sh` passes
