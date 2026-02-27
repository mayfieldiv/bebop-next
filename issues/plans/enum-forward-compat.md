# Plan: Enum Forward Compatibility (Accept Unknown Discriminators)

**Status:** Draft
**Depends on:** None
**Blocks:** None

## Problem Statement

The Rust plugin generates `#[repr(T)]` enums with a `TryFrom<T>` implementation that **rejects** unknown discriminator values by returning `DecodeError::InvalidEnum`. This makes enums a breaking change hazard: if a new variant is added to a schema, old code that hasn't been regenerated will fail to decode any message containing the new variant.

Both the C and Swift plugins handle this differently:

- **C plugin:** Enums are `typedef enum` (integers). Unknown values round-trip transparently since there's no validation.
- **Swift plugin:** Enums are `struct` wrappers around `RawRepresentable`, so unknown raw values are preserved.

The Rust plugin's unions already handle this correctly via the `Unknown(u8, Cow<'buf, [u8]>)` variant (gen_union.rs:97), providing a precedent within the codebase.

## Current Code Analysis

### `gen_enum.rs:86-107` — Enum Definition

```rust
output.push_str(&format!("#[repr({})]\n", base_type));
output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
output.push_str(&format!("{} enum {} {{\n", vis, name));
for m in members {
  output.push_str(&format!("  {} = {},\n", mname, formatted_value));
}
output.push_str("}\n\n");
```

This generates a Rust enum with `#[repr(u32)]` (or similar), which by definition cannot hold values outside the defined variants.

### `gen_enum.rs:109-139` — TryFrom Implementation

```rust
output.push_str(&format!(
  "impl ::core::convert::TryFrom<{}> for {} {{\n", base_type, name
));
output.push_str("  type Error = DecodeError;\n");
// ...match arms for known values...
output.push_str(&format!(
  "      _ => ::core::result::Result::Err(DecodeError::InvalidEnum {{ type_name: \"{}\", value: value as u64 }}),\n",
  name
));
```

The wildcard arm returns an error, making unknown values fatal.

### `gen_enum.rs:184-199` — BebopDecode

```rust
output.push_str(&format!("    let value = reader.{}()?;\n", read_method));
output.push_str("    ::core::convert::TryFrom::try_from(value)\n");
```

Decode reads the integer, then calls `TryFrom` which can fail. The `?` propagation causes the entire containing message/struct decode to fail.

### `gen_enum.rs:164-181` — BebopEncode

```rust
output.push_str(&format!(
  "    writer.{}(*self as {});\n", write_method, base_type
));
```

Encoding casts the enum variant to its base type. This only works because `#[repr(T)]` guarantees that `*self as u32` produces the discriminator value.

### Why `#[repr(T)]` Was Used

The current code uses `#[repr(u32)]` (or similar) for one specific reason: the **encode path** uses `*self as u32` to extract the discriminator. Without `#[repr(T)]`, casting an enum to an integer via `as` is not guaranteed to produce the discriminator value. The `repr` attribute makes the discriminator layout well-defined, enabling this cast.

However, this constraint only matters if we want to use `as` casting for encoding. If we encode differently (e.g., via a `match` statement or by storing the raw value), `#[repr(T)]` becomes unnecessary.

### Flags Enums (gen_enum.rs:206-321)

Flags enums are already forward-compatible! They use a newtype struct pattern:

```rust
pub struct MyFlags(pub u32);
```

The `BebopFlags` trait's `from_bits_retain` accepts any value. Only `from_bits()` validates (returning `Option`). The blanket `BebopDecode` impl (traits.rs:227-234) calls `from_bits_retain`, preserving unknown bits.

## Design Options

### Option A: Newtype Struct Pattern (Recommended)

Replace the `#[repr(T)]` enum with a newtype struct, mirroring the flags pattern:

```rust
// Generated for: enum Color { Red = 1; Green = 2; Blue = 3; }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color(pub u32);

#[allow(non_upper_case_globals)]
impl Color {
  pub const Red: Self = Self(1);
  pub const Green: Self = Self(2);
  pub const Blue: Self = Self(3);

  pub const FIXED_ENCODED_SIZE: usize = 4;

  /// Returns `true` if this value matches a known variant.
  pub fn is_known(self) -> bool {
    matches!(self.0, 1 | 2 | 3)
  }
}
```

**Encode/Decode** become trivial since the inner value is always valid:

```rust
impl BebopEncode for Color {
  fn encode(&self, writer: &mut BebopWriter) {
    writer.write_u32(self.0);
  }
  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }
}

impl<'buf> BebopDecode<'buf> for Color {
  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
    ::core::result::Result::Ok(Self(reader.read_u32()?))
  }
}
```

**TryFrom and From** are replaced:

```rust
// From<base_type> — always succeeds
impl ::core::convert::From<u32> for Color {
  fn from(value: u32) -> Self { Self(value) }
}

// From<Color> for base_type
impl ::core::convert::From<Color> for u32 {
  fn from(value: Color) -> u32 { value.0 }
}
```

**Strengths:**

- Consistent with the existing flags enum pattern
- Simple implementation — closely mirrors `generate_flags()`
- Forward-compatible: any `u32` value decodes and round-trips
- Infallible decode (no `TryFrom`, no `InvalidEnum`)

**Weaknesses:**

- Loss of exhaustive `match` — users can't use Rust's exhaustiveness checking
- `Color::Red` is an associated constant, not a variant — affects pattern matching ergonomics
- Can't destructure in `match` arms, must compare: `x if x == Color::Red =>`

### Option B: Regular Enum with `Unknown(T)` Variant (No `repr`)

Drop `#[repr(T)]` entirely and add an `Unknown(u32)` variant to hold unrecognized values:

```rust
// Generated for: enum Color { Red = 1; Green = 2; Blue = 3; }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Color {
  Red,
  Green,
  Blue,
  /// An unknown discriminator value from a newer schema version.
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
```

Without `#[repr(T)]`, we can't use `*self as u32` for encoding. Instead, encode/decode use explicit match:

```rust
impl BebopEncode for Color {
  fn encode(&self, writer: &mut BebopWriter) {
    writer.write_u32(self.discriminator());
  }
  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }
}

impl<'buf> BebopDecode<'buf> for Color {
  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {
    let value = reader.read_u32()?;
    ::core::result::Result::Ok(match value {
      1 => Self::Red,
      2 => Self::Green,
      3 => Self::Blue,
      v => Self::Unknown(v),
    })
  }
}
```

**From conversions:**

```rust
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
```

**Strengths:**

- Preserves Rust `match` ergonomics and exhaustiveness checking (with `Unknown(_)` as the catchall)
- `Color::Red` is a real enum variant — works naturally in patterns
- Mirrors how unions handle unknown discriminators (`Unknown(u8, Cow<'buf, [u8]>)`)
- Users can destructure: `match color { Color::Red => ..., Color::Unknown(v) => ... }`
- Forward-compatible: unknown values decode into `Unknown(v)` and round-trip correctly

**Weaknesses:**

- `Unknown(v)` is visible in the API — users must handle it in every `match`
- Slightly more generated code for the encode `match` (one arm per variant + Unknown)
- `Unknown(42)` could collide with a known variant: `Color::Unknown(1)` and `Color::Red` would be semantically the same but not `==` unless we add custom `PartialEq`. This is a real correctness hazard — see analysis below.
- Variants don't carry their discriminator values visibly (no `Red = 1` in the source)

**The `Unknown` collision problem:**

If a user constructs `Color::Unknown(1)`, it represents the same wire value as `Color::Red`, but `Color::Unknown(1) != Color::Red` with derived `PartialEq`. This can be mitigated by:

1. **Custom `PartialEq`/`Eq`/`Hash`:** Compare by discriminator value instead of variant. This means `Color::Unknown(1) == Color::Red`. Downside: surprising behavior, `Hash` inconsistency with the derived version.
2. **Normalize in `From<u32>`:** The `From` impl and `decode()` always return known variants for known values, so `Unknown(1)` can only be constructed manually. Document that `Unknown` should not be constructed with known values.
3. **Accept the gap:** Document that `Unknown` is for forward compatibility only and constructing it with known values is a logic error. The derived `PartialEq` works correctly for all values that come from decoding.

Option 2 is cleanest: decode always normalizes, so the collision can only happen via manual construction, which is the user's responsibility.

### Option C: Keep `#[repr(T)]` Enum, Wrap in Option

```rust
#[repr(u32)]
pub enum KnownColor { Red = 1, Green = 2, Blue = 3 }

pub struct Color { raw: u32 }
impl Color {
  pub fn known(&self) -> Option<KnownColor> { ... }
  pub fn raw(&self) -> u32 { self.raw }
}
```

This doubles the type surface and makes matching awkward. Not recommended.

### Option D: Non-exhaustive Enum with Fallback

```rust
#[non_exhaustive]
#[repr(u32)]
pub enum Color { Red = 1, Green = 2, Blue = 3 }
```

`#[non_exhaustive]` prevents exhaustive matching but does NOT allow unknown values to be stored. Decode would still need to error or return a sentinel. This doesn't solve the core problem.

## Recommended Approach: Option A (Newtype Struct)

After evaluating all options, Option A (newtype struct) is recommended for the following reasons:

### Why Not Option B (Regular Enum with `Unknown(T)`)

Option B is appealing because it preserves `match` ergonomics and mirrors the union `Unknown` pattern. However, the **discriminator collision problem** (`Color::Unknown(1) != Color::Red` when they represent the same wire value) introduces a correctness footgun that requires either custom trait impls or careful documentation. The union `Unknown` variant doesn't have this problem because union discriminators are single bytes with a body, making manual construction less likely.

Option B also generates more code per variant (a match arm in both encode and decode), while Option A's encode/decode are variant-count-independent.

### Why Option A

- **Consistent with flags enums** — the newtype struct pattern is already established in the codebase
- **No collision hazard** — `Color(1)` is always `Color(1)`, whether the user constructs it or it comes from decode
- **Simpler implementation** — closely mirrors `generate_flags()`, ~70% shared logic
- **Infallible decode** — no error path, no match statement
- **Smaller generated code** — variant-count-independent encode/decode

The trade-off (loss of `match` exhaustiveness) is the *entire point* of forward compatibility. If exhaustiveness checking fired on new variants, the code wouldn't compile — which is exactly the problem we're solving.

### Changes Required

#### 1. `gen_enum.rs:66-202` — Replace `generate_enum()`

The entire `generate_enum()` function is rewritten. The new version closely follows the existing `generate_flags()` pattern (lines 206-321) but without the bitwise operator impls and `BebopFlags` trait:

```rust
fn generate_enum(
  def: &DefinitionDescriptor,
  enum_def: &EnumDef,
  output: &mut String,
  name: &str,
  vis: &str,
  base_type: &str,
  read_method: &str,
  write_method: &str,
  byte_size: usize,
  is_signed: bool,
  base_kind: TypeKind,
) -> Result<(), GeneratorError> {
  let members = enum_def.members.as_deref().unwrap_or(&[]);

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Struct definition (newtype)
  output.push_str(
    "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]\n",
  );
  output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
  output.push_str(&format!("{} struct {}({} {});\n\n", vis, name, vis, base_type));

  // Associated constants
  output.push_str(&format!("#[allow(non_upper_case_globals)]\nimpl {} {{\n", name));
  output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n", byte_size
  ));

  for m in members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    emit_doc_comment(output, &m.documentation);
    emit_deprecated(output, &m.decorators);
    let formatted_value = if is_signed {
      format_signed_value(mvalue, base_kind)
    } else {
      format!("{}", mvalue)
    };
    output.push_str(&format!(
      "  pub const {}: Self = Self({});\n", mname, formatted_value
    ));
  }

  // is_known() helper
  output.push_str("\n  /// Returns `true` if this value matches a known variant.\n");
  output.push_str("  pub fn is_known(self) -> bool {\n");
  if members.is_empty() {
    output.push_str("    false\n");
  } else {
    output.push_str("    matches!(self.0, ");
    for (i, m) in members.iter().enumerate() {
      if i > 0 { output.push_str(" | "); }
      let mvalue = m.value.unwrap_or(0);
      let formatted = if is_signed {
        format_signed_value(mvalue, base_kind)
      } else {
        format!("{}", mvalue)
      };
      output.push_str(&formatted);
    }
    output.push_str(")\n");
  }
  output.push_str("  }\n");

  output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n", name
  ));
  output.push_str("}\n\n");

  // From<base_type> for Name (infallible)
  output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n", base_type, name
  ));
  output.push_str(&format!(
    "  fn from(value: {}) -> Self {{ Self(value) }}\n", base_type
  ));
  output.push_str("}\n\n");

  // From<Name> for base_type
  output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n", name, base_type
  ));
  output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value.0 }}\n", name, base_type
  ));
  output.push_str("}\n\n");

  // BebopEncode
  output.push_str(&format!("impl BebopEncode for {} {{\n", name));
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n", name
  ));
  output.push_str(&format!("    writer.{}(self.0);\n", write_method));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n", name
  ));
  output.push_str("  }\n\n");
  output.push_str("  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }\n");
  output.push_str("}\n\n");

  // BebopDecode
  output.push_str(&format!("impl<'buf> BebopDecode<'buf> for {} {{\n", name));
  output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n", name
  ));
  output.push_str(&format!(
    "    let value = reader.{}()?;\n", read_method
  ));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n", name
  ));
  output.push_str("    ::core::result::Result::Ok(Self(value))\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  Ok(())
}
```

#### 2. `error.rs` — Remove `InvalidEnum` Variant (Optional)

If no other code uses `DecodeError::InvalidEnum`, it can be removed. However, user code in insertion points may reference it, so it's safer to keep it initially and deprecate it.

#### 3. `mod.rs` — LifetimeAnalysis

No changes needed. Enums never have lifetimes; the newtype struct pattern doesn't change this.

#### 4. `type_mapper.rs` — Enum Type References

No changes needed. Enums are referenced by their type name in field types, and the newtype struct has the same name and the same base type encoding. The `is_fixed_scalar()` check and `fixed_size()` lookups all go through `TypeKind::Defined` → FQN → `enum_fqns`, which still works.

#### 5. Integration Tests

The integration tests in `tests/integration.rs` that test enum round-trips will need updating. The match syntax changes from:

```rust
// Old:
match color {
  Color::Red => { ... }
  Color::Green => { ... }
  _ => unreachable!(),
}

// New:
if color == Color::Red { ... }
else if color == Color::Green { ... }
```

Add a new test for unknown enum values:

```rust
#[test]
fn enum_unknown_value_round_trips() {
  let unknown = Color(999);
  let bytes = unknown.to_bytes();
  let decoded = Color::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, unknown);
  assert!(!decoded.is_known());
}
```

## Cost-Benefit Analysis

### Benefits

- **Forward compatibility:** Old code can decode messages with new enum variants without error
- **Wire format preservation:** Unknown values round-trip perfectly
- **Consistency:** Matches behavior of C plugin, Swift plugin, and Rust union's `Unknown` variant
- **Simpler decode path:** No `TryFrom` error propagation, no `DecodeError::InvalidEnum`
- **Consistency within codebase:** Non-flags enums now use the same newtype pattern as flags enums

### Costs

- **API change:** `Color::Red` changes from an enum variant to an associated constant. Code using `match` statements must switch to `if`/`else if` or use constant patterns
- **Loss of exhaustive matching:** Rust's `match` exhaustiveness checking is lost. Users won't get a compile error when new variants are added — but this is the *entire point* of forward compatibility
- **Slightly larger generated code:** The `is_known()` helper adds a few lines
- **Pattern matching ergonomics:** Cannot destructure `Color::Red` in patterns, must use `Color::Red` as a constant pattern: `x if x == Color::Red =>`

### Generated Code Size Impact

- Current (enum + TryFrom + From + encode + decode): ~60 lines
- Proposed (newtype + consts + From×2 + is_known + encode + decode): ~55 lines
- **Net:** Slightly smaller

## Relationship to Flags Enums

The proposed change makes non-flags enums structurally identical to flags enums (both are newtype structs with associated constants). The difference:

- **Flags:** has `BebopFlags` trait, bitwise operators, `from_bits()`/`from_bits_retain()`
- **Non-flags:** has `is_known()`, `From<T>` conversions

This could be further unified: a shared trait or a code-sharing function in `gen_enum.rs` that generates the common newtype pattern. The `generate_flags()` and `generate_enum()` functions share ~70% of their logic after this change.

## Test Plan

1. Update all existing enum unit tests in `mod.rs` for the new pattern
2. Add test: unknown enum value decodes successfully
3. Add test: unknown enum value round-trips
4. Add test: `is_known()` returns true for defined variants, false for unknown
5. Add test: `From<u32>` and `From<Color>` conversions
6. Update integration tests for new matching syntax
7. Add no_std integration test for unknown enum value
