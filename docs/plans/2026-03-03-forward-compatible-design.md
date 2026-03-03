# Design: `@forward_compatible` Decorator

**Date:** 2026-03-03

## Summary

A new built-in `@forward_compatible` schema decorator that opts individual types into tolerating unknown values during decode. Without the decorator, all types are strict — they reject unrecognized discriminators, field tags, or bit patterns. With the decorator, unknown values are preserved and can round-trip through encode/decode.

This makes the current behavior (which is forward-compatible for unions, messages, and flags) opt-in rather than default.

## Decorator Definition

Added to `bebop/schemas/bebop/decorators.bop`:

```bebop
#decorator(forward_compatible) {
    targets = ENUM | UNION | MESSAGE
}
```

`ENUM` covers both regular enums and `@flags` enums. `STRUCT` is excluded — structs have fixed byte layouts and are not extensible.

## Behavior Matrix

| Type | Default (strict) | `@forward_compatible` |
|---|---|---|
| Enum | `#[repr(T)]` enum, `TryFrom` rejects unknown discriminators | Regular enum with `Unknown(T)` variant, infallible decode |
| Flags | `from_bits()` rejects unknown bit patterns | `from_bits_retain()` accepts any bits (current behavior) |
| Union | Rejects unknown discriminators | `Unknown(u8, Cow<'buf, [u8]>)` variant preserves unknown branches (current behavior) |
| Message | Rejects unknown field tags | Skips unknown fields via length prefix (current behavior) |
| Struct | Fixed layout, no extensibility | N/A — decorator rejected by compiler |

The key insight: **current behavior for unions, messages, and flags IS the forward-compatible behavior.** This design makes strict the new default and `@forward_compatible` restores today's behavior for those types.

## Detailed Design

### Enums — Strict (Default, No Change)

```rust
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Red = 1,
    Green = 2,
    Blue = 3,
}

impl ::core::convert::TryFrom<u32> for Color {
    type Error = DecodeError;
    fn try_from(value: u32) -> ::core::result::Result<Self, DecodeError> {
        match value {
            1 => Ok(Self::Red),
            2 => Ok(Self::Green),
            3 => Ok(Self::Blue),
            _ => Err(DecodeError::InvalidEnum { type_name: "Color", value: value as u64 }),
        }
    }
}

impl BebopEncode for Color {
    fn encode(&self, writer: &mut BebopWriter) {
        writer.write_u32(*self as u32);
    }
    fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }
}

impl<'buf> BebopDecode<'buf> for Color {
    fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
        let value = reader.read_u32()?;
        ::core::convert::TryFrom::try_from(value)
    }
}
```

No changes to the current `gen_enum.rs` strict path.

### Enums — `@forward_compatible`

```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Blue,
    /// A discriminator value not recognized by this version of the schema.
    Unknown(u32),
}

impl Color {
    pub const FIXED_ENCODED_SIZE: usize = 4;

    /// Returns the raw discriminator value.
    pub fn discriminator(self) -> u32 {
        match self {
            Self::Red => 1,
            Self::Green => 2,
            Self::Blue => 3,
            Self::Unknown(v) => v,
        }
    }

    /// Returns `true` if this value matches a known variant.
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

impl ::core::convert::From<u32> for Color {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Blue,
            v => Self::Unknown(v),
        }
    }
}

impl ::core::convert::From<Color> for u32 {
    fn from(value: Color) -> u32 { value.discriminator() }
}

impl BebopEncode for Color {
    fn encode(&self, writer: &mut BebopWriter) {
        writer.write_u32(self.discriminator());
    }
    fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }
}

impl<'buf> BebopDecode<'buf> for Color {
    fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
        let value = reader.read_u32()?;
        ::core::result::Result::Ok(Self::from(value))
    }
}
```

No `#[repr(T)]`. No `TryFrom`. Infallible `From` normalizes known values. `Unknown(T)` catches the rest. Derived `PartialEq`/`Eq`/`Hash` (no custom impls).

### Flags — Strict (New Default)

Currently, the blanket `BebopDecode` impl for `BebopFlags` types calls `from_bits_retain()`, which accepts any bit pattern. For strict flags, decode should reject unknown bits.

The blanket impl in `traits.rs:227-234`:

```rust
impl<'buf, T: BebopFlags> BebopDecode<'buf> for T {
    fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
        let bits = <Self::Bits as BebopDecode>::decode(reader)?;
        Ok(Self::from_bits_retain(bits))
    }
}
```

**Option 1: Split the blanket impl.** Remove the blanket `BebopDecode` impl for `BebopFlags`. Instead, generate `BebopDecode` explicitly for each flags type:
- Strict: call `from_bits()`, return `DecodeError::InvalidFlags` if `None`
- Forward-compatible: call `from_bits_retain()`

This is the cleanest approach since the generator already controls whether `@forward_compatible` is present.

**Option 2: Add a trait method.** Add `fn is_strict() -> bool` to `BebopFlags` and branch in the blanket impl. Workable but couples the trait to the decorator concept.

**Recommendation: Option 1.** Remove the blanket `BebopDecode` impl for flags and generate it per-type. The blanket `BebopEncode` impl can stay since encoding is the same either way.

Generated strict flags decode:

```rust
impl<'buf> BebopDecode<'buf> for MyFlags {
    fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
        let bits = reader.read_u32()?;
        Self::from_bits(bits).ok_or(DecodeError::InvalidFlags {
            type_name: "MyFlags",
            bits: bits as u64,
        })
    }
}
```

Generated forward-compatible flags decode:

```rust
impl<'buf> BebopDecode<'buf> for MyFlags {
    fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
        let bits = reader.read_u32()?;
        ::core::result::Result::Ok(Self::from_bits_retain(bits))
    }
}
```

### Unions — Strict (New Default)

Remove the `Unknown(u8, Cow<'buf, [u8]>)` variant. Reject unknown discriminators:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MyUnion {       // no 'buf if no branch needs it
    Foo(Foo),
    Bar(Bar),
}
```

Decode unknown discriminator returns an error:

```rust
_ => ::core::result::Result::Err(DecodeError::InvalidUnion {
    type_name: "MyUnion",
    discriminator,
}),
```

**Lifetime impact:** Without the `Unknown` variant (which always needs `Cow<'buf, [u8]>`), a union only needs `'buf` if any of its branches need it. This changes the `LifetimeAnalysis` — unions are no longer unconditionally added to `lifetime_fqns`.

### Unions — `@forward_compatible` (Current Behavior)

No changes from today's generated code. The `Unknown(u8, Cow<'buf, [u8]>)` variant is present, decode catches unknown discriminators, and the union always has `'buf`.

### Messages — Strict (New Default)

Currently, the decode loop's default arm skips unknown fields using the message length. For strict messages, this becomes an error.

Current (forward-compatible) decode pattern in `gen_message.rs`:

```rust
_ => {
    reader.seek(start + length)?;
    break;
}
```

Strict decode pattern:

```rust
tag => {
    return ::core::result::Result::Err(DecodeError::InvalidField {
        type_name: "MyMessage",
        tag,
    });
}
```

Note: the behavior for fields marked `@deprecated` in the schema should still skip gracefully even in strict mode. This only applies to truly unknown tags — tags that don't appear in the schema at all.

### Messages — `@forward_compatible` (Current Behavior)

No changes from today. Unknown field tags are skipped via the length prefix.

## Error Variants

New variants added to `DecodeError`:

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
    UnexpectedEof { needed: usize, available: usize },
    InvalidUtf8,
    InvalidEnum { type_name: &'static str, value: u64 },
    InvalidUnion { type_name: &'static str, discriminator: u8 },   // NEW
    InvalidField { type_name: &'static str, tag: u8 },             // NEW
    InvalidFlags { type_name: &'static str, bits: u64 },           // NEW
}
```

Also add `#[non_exhaustive]` to allow future variants.

## Generator Changes

### Shared Helper

```rust
fn has_decorator(def: &DefinitionDescriptor, fqn: &str) -> bool {
    def.decorators.as_ref().map_or(false, |decs| {
        decs.iter().any(|d| d.fqn.as_deref() == Some(fqn))
    })
}
```

### `gen_enum.rs`

```rust
let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");

if is_flags {
    generate_flags(def, enum_def, output, ..., is_forward_compatible)?;
} else if is_forward_compatible {
    generate_forward_compatible_enum(def, enum_def, output, ...)?;
} else {
    generate_strict_enum(def, enum_def, output, ...)?;  // current code, renamed
}
```

- `generate_strict_enum()`: Current `generate_enum()`, unchanged.
- `generate_forward_compatible_enum()`: New function. No `#[repr]`, `Unknown(T)` variant, `discriminator()` method, infallible `From`, match-based encode/decode.
- `generate_flags()`: Takes `is_forward_compatible` param. Generates `BebopDecode` per-type (strict uses `from_bits()` + error, forward-compatible uses `from_bits_retain()`). Remove the blanket `BebopDecode` impl from `traits.rs`.

### `gen_union.rs`

```rust
let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");
```

Conditionally:
- Emit `Unknown(u8, Cow<'buf, [u8]>)` variant (only if forward-compatible)
- Emit `into_owned()` arm for Unknown (only if forward-compatible)
- Emit encode/decode arms for Unknown (only if forward-compatible)
- Emit error arm for unknown discriminator (only if strict)
- Apply `'buf` lifetime: always if forward-compatible, conditionally based on branch analysis if strict

### `gen_message.rs`

```rust
let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");
```

Conditionally:
- Emit skip-unknown-fields logic (forward-compatible) or error-on-unknown-field (strict)

### `mod.rs` — LifetimeAnalysis

Currently, all unions are unconditionally added to `lifetime_fqns` (because the `Unknown` variant always needs `'buf`). This changes:

- Strict unions: only in `lifetime_fqns` if any branch's inner type needs a lifetime
- Forward-compatible unions: always in `lifetime_fqns` (the `Unknown` variant needs `Cow<'buf, [u8]>`)

The `LifetimeAnalysis` fixpoint iteration needs access to decorator info to make this determination.

### `traits.rs`

Remove the blanket `BebopDecode` impl for `BebopFlags`. Keep the blanket `BebopEncode` impl (encoding is the same regardless of forward-compatibility).

## Test Plan

1. **Enum strict:** Verify unknown discriminator returns `DecodeError::InvalidEnum` (existing behavior)
2. **Enum forward-compatible:** Verify unknown discriminator decodes to `Unknown(v)`, round-trips correctly
3. **Enum forward-compatible `is_known()`:** Returns true for defined variants, false for unknown
4. **Flags strict:** Verify unknown bits return `DecodeError::InvalidFlags`
5. **Flags forward-compatible:** Verify unknown bits are retained (existing behavior)
6. **Union strict:** Verify unknown discriminator returns `DecodeError::InvalidUnion`
7. **Union forward-compatible:** Verify unknown discriminator decodes to `Unknown(disc, data)`, round-trips (existing behavior)
8. **Union strict lifetime:** Verify union without lifetime-bearing branches generates `MyUnion` (no `'buf`)
9. **Message strict:** Verify unknown field tag returns `DecodeError::InvalidField`
10. **Message forward-compatible:** Verify unknown fields are skipped (existing behavior)
11. **Decorator validation:** Verify `@forward_compatible` on a struct is rejected by the compiler
12. **Integration:** Full round-trip tests for all four type kinds in both modes
