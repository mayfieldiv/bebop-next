# `@forward_compatible` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `@forward_compatible` schema decorator that opts enums, flags, unions, and messages into tolerating unknown values during decode. Without it, all types are strict (reject unknowns).

**Architecture:** The current behavior (forward-compatible) becomes opt-in via `@forward_compatible`. Strict becomes the new default for unions, messages, and flags. Enums are already strict. Changes touch the runtime (`error.rs`, `traits.rs`) and four generators (`gen_enum.rs`, `gen_union.rs`, `gen_message.rs`, `mod.rs`).

**Tech Stack:** Rust, bebop schema language, bebopc-gen-rust plugin

**Design doc:** `docs/plans/2026-03-03-forward-compatible-design.md`

**Process rules:**
- **Red-green TDD:** For every task with tests, write the failing test FIRST, run it to confirm it fails (red), then implement the minimal code to make it pass (green). Never write implementation before seeing the test fail.
- **Formatting:** Run `cargo fmt` after every implementation step, before committing.
- **Linting:** Run `cargo clippy -- -D warnings` after every implementation step, before committing. Fix any warnings.
- **Atomic commits:** Each commit must compile, pass all tests, and have clean fmt/clippy.

---

### Task 1: Add New Error Variants to Runtime

**Files:**
- Modify: `plugins/rust/runtime/src/error.rs`

**Step 1: Write the failing test**

No separate test file — verify compilation in step 4.

**Step 2: Add error variants and `#[non_exhaustive]`**

Replace the entire `error.rs` with:

```rust
use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
  UnexpectedEof { needed: usize, available: usize },
  InvalidUtf8,
  InvalidEnum { type_name: &'static str, value: u64 },
  InvalidUnion { type_name: &'static str, discriminator: u8 },
  InvalidField { type_name: &'static str, tag: u8 },
  InvalidFlags { type_name: &'static str, bits: u64 },
}

impl fmt::Display for DecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnexpectedEof { needed, available } => {
        write!(
          f,
          "unexpected eof: needed {} bytes, {} available",
          needed, available
        )
      }
      Self::InvalidUtf8 => write!(f, "invalid utf-8 in string"),
      Self::InvalidEnum { type_name, value } => {
        write!(f, "invalid {} value: {}", type_name, value)
      }
      Self::InvalidUnion { type_name, discriminator } => {
        write!(f, "invalid {} discriminator: {}", type_name, discriminator)
      }
      Self::InvalidField { type_name, tag } => {
        write!(f, "invalid {} field tag: {}", type_name, tag)
      }
      Self::InvalidFlags { type_name, bits } => {
        write!(f, "invalid {} bits: {:#x}", type_name, bits)
      }
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}
```

**Step 3: Verify runtime compiles**

Run: `cd plugins/rust/runtime && cargo check`
Expected: compiles with no errors

**Step 4: Verify plugin compiles**

Run: `cd plugins/rust && cargo check`
Expected: compiles with no errors

**Step 5: Commit**

Message: `feat(runtime): add DecodeError variants for strict mode`

---

### Task 2: Remove Blanket `BebopDecode` for `BebopFlags`

**Files:**
- Modify: `plugins/rust/runtime/src/traits.rs:227-234`

**Step 1: Delete the blanket impl**

Remove these lines from `traits.rs` (lines 227-234):

```rust
impl<'buf, T> BebopDecode<'buf> for T
where
  T: BebopFlags,
{
  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    Ok(Self::from_bits_retain(T::Bits::decode_bits(reader)?))
  }
}
```

**Step 2: Verify runtime compiles**

Run: `cd plugins/rust/runtime && cargo check`
Expected: compiles (nothing in the runtime uses this blanket impl directly)

**Step 3: Run existing tests**

Run: `cd plugins/rust && cargo test`
Expected: all existing tests pass (tests check string output, not compiled generated code)

**Step 4: Commit**

Message: `refactor(runtime): remove blanket BebopDecode for BebopFlags`

---

### Task 3: Add `has_decorator` Helper and Update LifetimeAnalysis

**Files:**
- Modify: `plugins/rust/src/generator/mod.rs`

**Step 1: Add the `has_decorator` helper**

Add this public function after `emit_deprecated` (after line 626):

```rust
/// Returns `true` if the definition has a decorator with the given FQN.
pub fn has_decorator(def: &DefinitionDescriptor, fqn: &str) -> bool {
  def.decorators.as_ref().map_or(false, |decs| {
    decs.iter().any(|d| d.fqn.as_deref() == Some(fqn))
  })
}
```

**Step 2: Update LifetimeAnalysis — unions conditional**

In `build_all()`, replace the block at lines 114-117:

```rust
      if def.kind == Some(DefinitionKind::Union) {
        // Unions always need lifetime (Unknown variant uses Cow<'buf, [u8]>)
        analysis.lifetime_fqns.insert(fqn.clone());
      }
```

With:

```rust
      if def.kind == Some(DefinitionKind::Union) {
        // Forward-compatible unions always need lifetime (Unknown variant uses Cow<'buf, [u8]>).
        // Strict unions only need lifetime if their branches do (resolved in the fixpoint loop).
        if has_decorator(def, "bebop.forward_compatible") {
          analysis.lifetime_fqns.insert(fqn.clone());
        }
      }
```

Also update the fixpoint loop's union arm at line 151:

```rust
          Some(DefinitionKind::Union) => true,
```

Replace with:

```rust
          Some(DefinitionKind::Union) => {
            // Forward-compatible unions already added above.
            // Strict unions need lifetime only if any branch does.
            if has_decorator(def, "bebop.forward_compatible") {
              true
            } else {
              def
                .union_def
                .as_ref()
                .and_then(|ud| ud.branches.as_deref())
                .is_some_and(|branches| {
                  branches.iter().any(|b| {
                    let branch_fqn = b
                      .type_ref_fqn
                      .as_deref()
                      .or(b.inline_fqn.as_deref());
                    branch_fqn.is_some_and(|fqn| analysis.lifetime_fqns.contains(fqn))
                  })
                })
            }
          }
```

**Step 3: Write test for strict union without lifetime**

Add to the test module in `mod.rs`:

```rust
  #[test]
  fn strict_union_omits_lifetime_when_branches_are_scalar() {
    // Union with only scalar branches and no @forward_compatible → no 'buf
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Inner")),
      fqn: Some(Cow::Borrowed("test.Inner")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let strict_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("StrictUnion")),
      fqn: Some(Cow::Borrowed("test.StrictUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("inner")),
          type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict.bop")),
      definitions: Some(vec![inner_struct, strict_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    assert!(
      !analysis.lifetime_fqns.contains("test.StrictUnion"),
      "strict union with scalar-only branches should NOT need lifetime"
    );
  }

  #[test]
  fn forward_compatible_union_always_has_lifetime() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Inner")),
      fqn: Some(Cow::Borrowed("test.Inner")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let fc_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("FcUnion")),
      fqn: Some(Cow::Borrowed("test.FcUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("inner")),
          type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
          ..Default::default()
        }]),
      }),
      decorators: Some(vec![DecoratorUsage {
        fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc.bop")),
      definitions: Some(vec![inner_struct, fc_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    assert!(
      analysis.lifetime_fqns.contains("test.FcUnion"),
      "forward-compatible union should always need lifetime"
    );
  }
```

Note: the test import block at line 633-636 needs `DecoratorUsage` added:

```rust
  use crate::generated::{
    ConstDef, DecoratorUsage, DefinitionDescriptor, DefinitionKind, EnumDef, EnumMemberDescriptor,
    FieldDescriptor, LiteralKind, LiteralValue, MessageDef, SchemaDescriptor, StructDef,
    TypeDescriptor, TypeKind, UnionBranchDescriptor, UnionDef, Visibility,
  };
```

**Step 4: Run tests**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass, including the two new ones

**Step 5: Commit**

Message: `feat(generator): add has_decorator helper and conditional union lifetime`

---

### Task 4: Forward-Compatible Enum Generation

**Files:**
- Modify: `plugins/rust/src/generator/gen_enum.rs`

**Step 1: Write failing test**

Add to the test module in `mod.rs`:

```rust
  fn forward_compatible_decorator() -> Vec<DecoratorUsage<'static>> {
    vec![DecoratorUsage {
      fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
      ..Default::default()
    }]
  }

  #[test]
  fn forward_compatible_enum_emits_unknown_variant() {
    let fc_enum = DefinitionDescriptor {
      kind: Some(DefinitionKind::Enum),
      name: Some(Cow::Borrowed("Color")),
      fqn: Some(Cow::Borrowed("test.Color")),
      enum_def: Some(EnumDef {
        base_type: Some(TypeKind::Uint32),
        members: Some(vec![
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("Red")),
            value: Some(1),
            ..Default::default()
          },
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("Green")),
            value: Some(2),
            ..Default::default()
          },
        ]),
        is_flags: Some(false),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-enum.bop")),
      definitions: Some(vec![fc_enum]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should NOT have #[repr]
    assert!(!output.contains("#[repr("));
    // Should have Unknown variant
    assert!(output.contains("Unknown(u32)"));
    // Should have discriminator() method
    assert!(output.contains("fn discriminator(self)"));
    // Should have is_known() method
    assert!(output.contains("fn is_known("));
    // Should use From, not TryFrom
    assert!(output.contains("impl ::core::convert::From<u32> for Color"));
    assert!(!output.contains("TryFrom"));
    // Should use discriminator() in encode
    assert!(output.contains("self.discriminator()"));
    // Decode should use From
    assert!(output.contains("Self::from(value)"));
  }
```

**Step 2: Run test to verify it fails**

Run: `cd plugins/rust && cargo test forward_compatible_enum_emits_unknown_variant`
Expected: FAIL — the current generator ignores decorators on enums

**Step 3: Implement forward-compatible enum generation**

In `gen_enum.rs`, make these changes:

1. Add import for `has_decorator` at the top:

```rust
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis,
};
```

2. Update `generate()` function to branch on decorator (replace lines 41-59):

```rust
  let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");

  if is_flags {
    generate_flags(
      def, enum_def, output, &name, vis, base_type, read_method, write_method,
      byte_size, is_signed, base_kind, is_forward_compatible,
    )?;
  } else if is_forward_compatible {
    generate_forward_compatible_enum(
      def, enum_def, output, &name, vis, base_type, read_method, write_method,
      byte_size, is_signed, base_kind,
    )?;
  } else {
    generate_enum(
      def, enum_def, output, &name, vis, base_type, read_method, write_method,
      byte_size, is_signed, base_kind,
    )?;
  }
```

3. Add `generate_forward_compatible_enum()` function (new, after `generate_enum`):

```rust
/// Generate a forward-compatible enum with an Unknown(T) variant.
/// Used when @forward_compatible decorator is present.
#[allow(clippy::too_many_arguments)]
fn generate_forward_compatible_enum(
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

  // Enum definition — no #[repr]
  output
    .push_str("#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]\n");
  output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
  output.push_str(&format!("{} enum {} {{\n", vis, name));

  for m in members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    emit_doc_comment(output, &m.documentation);
    emit_deprecated(output, &m.decorators);
    output.push_str(&format!("  {},\n", mname));
  }
  output.push_str("  /// A discriminator value not recognized by this version of the schema.\n");
  output.push_str(&format!("  Unknown({}),\n", base_type));
  output.push_str("}\n\n");

  // ── impl block: FIXED_ENCODED_SIZE, discriminator(), is_known() ──
  output.push_str(&format!("impl {} {{\n", name));
  output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n\n",
    byte_size
  ));

  // discriminator()
  output.push_str(&format!(
    "  /// Returns the raw discriminator value.\n  pub fn discriminator(self) -> {} {{\n    match self {{\n",
    base_type
  ));
  for m in members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = if is_signed {
      format_signed_value(mvalue, base_kind)
    } else {
      format!("{}", mvalue)
    };
    output.push_str(&format!("      Self::{} => {},\n", mname, formatted_value));
  }
  output.push_str("      Self::Unknown(v) => v,\n");
  output.push_str("    }\n  }\n\n");

  // is_known()
  output.push_str("  /// Returns `true` if this value matches a known variant.\n");
  output.push_str("  pub fn is_known(&self) -> bool {\n");
  output.push_str("    !matches!(self, Self::Unknown(_))\n");
  output.push_str("  }\n");

  output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  // ── From<base_type> for Name (infallible) ──
  output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n",
    base_type, name
  ));
  output.push_str(&format!(
    "  fn from(value: {}) -> Self {{\n    match value {{\n",
    base_type
  ));
  for m in members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = if is_signed {
      format_signed_value(mvalue, base_kind)
    } else {
      format!("{}", mvalue)
    };
    output.push_str(&format!(
      "      {} => Self::{},\n",
      formatted_value, mname
    ));
  }
  output.push_str("      v => Self::Unknown(v),\n");
  output.push_str("    }\n  }\n}\n\n");

  // ── From<Name> for base_type ──
  output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n",
    name, base_type
  ));
  output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value.discriminator() }}\n",
    name, base_type
  ));
  output.push_str("}\n\n");

  // ── BebopEncode ──
  output.push_str(&format!("impl BebopEncode for {} {{\n", name));
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    name
  ));
  output.push_str(&format!(
    "    writer.{}(self.discriminator());\n",
    write_method
  ));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");
  output.push_str("  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }\n");
  output.push_str("}\n\n");

  // ── BebopDecode ──
  output.push_str(&format!("impl<'buf> BebopDecode<'buf> for {} {{\n", name));
  output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  output.push_str(&format!("    let value = reader.{}()?;\n", read_method));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  output.push_str("    ::core::result::Result::Ok(Self::from(value))\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  Ok(())
}
```

4. Update `generate_flags()` signature to accept `is_forward_compatible` and `read_method`/`write_method`, and emit `BebopDecode` at the end (after the bitwise operators, before `Ok(())`):

```rust
  // ── BebopDecode (generated per-type, not blanket) ──
  output.push_str(&format!("impl<'buf> BebopDecode<'buf> for {} {{\n", name));
  output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  output.push_str(&format!("    let bits = reader.{}()?;\n", read_method));
  if is_forward_compatible {
    output.push_str("    ::core::result::Result::Ok(Self::from_bits_retain(bits))\n");
  } else {
    output.push_str(&format!(
      "    Self::from_bits(bits).ok_or(DecodeError::InvalidFlags {{ type_name: \"{}\", bits: bits as u64 }})\n",
      name
    ));
  }
  output.push_str("  }\n");
  output.push_str("}\n\n");
```

The `generate_flags` signature becomes:

```rust
fn generate_flags(
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
  is_forward_compatible: bool,
) -> Result<(), GeneratorError> {
```

**Step 4: Run all tests**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass

**Step 5: Commit**

Message: `feat(gen_enum): forward-compatible enum with Unknown variant and per-type flags decode`

---

### Task 5: Strict Union Generation

**Files:**
- Modify: `plugins/rust/src/generator/gen_union.rs`

**Step 1: Write failing test**

Add to the test module in `mod.rs`:

```rust
  #[test]
  fn strict_union_rejects_unknown_discriminator() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let strict_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("StrictUnion")),
      fqn: Some(Cow::Borrowed("test.StrictUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict-union.bop")),
      definitions: Some(vec![inner_struct, strict_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should NOT have Unknown variant
    assert!(!output.contains("Unknown(u8,"));
    // Should NOT have 'buf on the enum (all branches are scalar structs)
    assert!(output.contains("pub enum StrictUnion {"));
    assert!(!output.contains("pub enum StrictUnion<'buf>"));
    // Should have InvalidUnion error in decode
    assert!(output.contains("InvalidUnion"));
  }

  #[test]
  fn forward_compatible_union_has_unknown_variant() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let fc_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("FcUnion")),
      fqn: Some(Cow::Borrowed("test.FcUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-union.bop")),
      definitions: Some(vec![inner_struct, fc_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have Unknown variant with Cow
    assert!(output.contains("Unknown(u8, alloc::borrow::Cow<'buf, [u8]>)"));
    // Should have 'buf
    assert!(output.contains("pub enum FcUnion<'buf>"));
    // Should NOT have InvalidUnion
    assert!(!output.contains("InvalidUnion"));
  }
```

**Step 2: Run tests to verify they fail**

Run: `cd plugins/rust && cargo test strict_union_rejects`
Expected: FAIL

**Step 3: Implement conditional union generation**

In `gen_union.rs`:

1. Add `has_decorator` import:

```rust
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis,
};
```

2. At the top of `generate()`, after line 61, add:

```rust
  let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");
```

3. Replace the lifetime computation (line 63-64):

```rust
  // Unions always have lifetime (Unknown variant uses alloc::borrow::Cow<'buf, [u8]>)
  let lt = "<'buf>";
```

With:

```rust
  let has_lifetime = if is_forward_compatible {
    true // Unknown variant always needs Cow<'buf, [u8]>
  } else {
    analysis.lifetime_fqns.contains(fqn)
  };
  let lt = if has_lifetime { "<'buf>" } else { "" };
```

4. Replace the Unknown variant emission (lines 95-97):

```rust
  // Unknown variant for forward compatibility
  output.push_str("  #[cfg_attr(feature = \"serde\", serde(skip))]\n");
  output.push_str("  Unknown(u8, alloc::borrow::Cow<'buf, [u8]>),\n");
```

With:

```rust
  if is_forward_compatible {
    output.push_str("  #[cfg_attr(feature = \"serde\", serde(skip))]\n");
    output.push_str("  Unknown(u8, alloc::borrow::Cow<'buf, [u8]>),\n");
  }
```

5. Make `TypeOwned` alias and `into_owned()` conditional on `has_lifetime` (wrap lines 100-133):

```rust
  if has_lifetime {
    // ── Type alias ────────────────────────────────────────────────
    output.push_str(&format!("{} type {}Owned = {}<'static>;\n\n", vis, name, name));

    // ── into_owned() ──────────────────────────────────────────────
    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
    output.push_str(&format!("  pub fn into_owned(self) -> {}Owned {{\n", name));
    output.push_str("    match self {\n");
    for b in &branch_infos {
      // ... existing branch logic ...
    }
    if is_forward_compatible {
      output.push_str(&format!(
        "      Self::Unknown(disc, data) => {}::Unknown(disc, alloc::borrow::Cow::Owned(data.into_owned())),\n",
        name
      ));
    }
    output.push_str("    }\n  }\n}\n\n");
  }
```

6. In the encode `match`, make the Unknown arm conditional (lines 156-158):

```rust
  if is_forward_compatible {
    output.push_str(
      "      Self::Unknown(disc, data) => { writer.write_byte(*disc); writer.write_raw(data); }\n",
    );
  }
```

7. In `encoded_size()`, make the Unknown arm conditional (line 176):

```rust
  if is_forward_compatible {
    output.push_str("      Self::Unknown(_, data) => wire::tagged_size(data.len()),\n");
  }
```

8. In the decode `match`, replace the Unknown catchall (lines 208-214):

```rust
  if is_forward_compatible {
    output.push_str("      _ => {\n");
    output.push_str("        let remaining = length - (reader.position() - start);\n");
    output.push_str("        let data = reader.read_raw_bytes(remaining)?;\n");
    output.push_str(
      "        ::core::result::Result::Ok(Self::Unknown(discriminator, alloc::borrow::Cow::Borrowed(data)))\n",
    );
    output.push_str("      }\n");
  } else {
    output.push_str(&format!(
      "      _ => ::core::result::Result::Err(DecodeError::InvalidUnion {{ type_name: \"{}\", discriminator }}),\n",
      name
    ));
  }
```

9. Update the `impl` blocks to use conditional lifetime (all `<'buf>` references become `lt`).

**Step 4: Run all tests**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass

**Step 5: Commit**

Message: `feat(gen_union): strict mode rejects unknown discriminators, conditional lifetime`

---

### Task 6: Strict Message Generation

**Files:**
- Modify: `plugins/rust/src/generator/gen_message.rs`

**Step 1: Write failing test**

Add to the test module in `mod.rs`:

```rust
  #[test]
  fn strict_message_rejects_unknown_field_tag() {
    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("StrictMsg")),
      fqn: Some(Cow::Borrowed("test.StrictMsg")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict-msg.bop")),
      definitions: Some(vec![msg]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have InvalidField error
    assert!(output.contains("InvalidField"));
    // Should NOT have reader.skip
    assert!(!output.contains("reader.skip("));
  }

  #[test]
  fn forward_compatible_message_skips_unknown_fields() {
    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("FcMsg")),
      fqn: Some(Cow::Borrowed("test.FcMsg")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-msg.bop")),
      definitions: Some(vec![msg]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have reader.skip (forward-compatible behavior)
    assert!(output.contains("reader.skip("));
    // Should NOT have InvalidField
    assert!(!output.contains("InvalidField"));
  }
```

**Step 2: Run tests to verify they fail**

Run: `cd plugins/rust && cargo test strict_message_rejects`
Expected: FAIL

**Step 3: Implement conditional message decode**

In `gen_message.rs`:

1. Add `has_decorator` import:

```rust
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis,
};
```

2. At the top of `generate()`, after line 57, add:

```rust
  let is_forward_compatible = has_decorator(def, "bebop.forward_compatible");
```

3. Replace the unknown-field handler (line 351):

```rust
  output.push_str("        _ => { reader.skip(end - reader.position())?; }\n");
```

With:

```rust
  if is_forward_compatible {
    output.push_str("        _ => { reader.skip(end - reader.position())?; }\n");
  } else {
    output.push_str(&format!(
      "        tag => {{ return ::core::result::Result::Err(DecodeError::InvalidField {{ type_name: \"{}\", tag }}); }}\n",
      name
    ));
  }
```

**Step 4: Run all tests**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass

**Step 5: Commit**

Message: `feat(gen_message): strict mode rejects unknown field tags`

---

### Task 7: Schema Decorator Definition

**Files:**
- Modify: `bebop/schemas/bebop/decorators.bop`

**Step 1: Add the decorator definition**

Append after the `@flags` decorator:

```bebop

/// Marks a type as forward-compatible with future schema versions.
///
/// Without this decorator, decoding a value with an unrecognized
/// discriminator, field tag, or bit pattern is an error. With this
/// decorator, unknown values are preserved and can round-trip.
///
/// - **enum**: Adds an `Unknown(T)` variant for unrecognized discriminators.
/// - **@flags enum**: Accepts unknown bit patterns without error.
/// - **union**: Adds an `Unknown` branch for unrecognized discriminators.
/// - **message**: Silently skips unrecognized field tags.
///
/// ```bebop
/// @forward_compatible
/// enum Status : uint32 {
///   OK = 0;
///   ERROR = 1;
/// }
/// ```
#decorator(forward_compatible) {
    targets = ENUM | UNION | MESSAGE
}
```

**Step 2: Commit**

Message: `feat(schema): add @forward_compatible decorator definition`

---

### Task 8: Update Existing Tests

**Files:**
- Modify: `plugins/rust/src/generator/mod.rs` (test module)

The existing `build_schema()` creates a union (`ResultUnion`) without `@forward_compatible`, so it's now a strict union. Several existing tests check for insertion points like `encode_switch:ResultUnion` and `decode_switch:ResultUnion`, which are only emitted when there's an `Unknown` variant (forward-compatible mode).

**Step 1: Update `build_schema()` to add `@forward_compatible` to the union**

In the `result_union` definition (around line 730), add the decorator:

```rust
    let result_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("ResultUnion")),
      fqn: Some(Cow::Borrowed("test.ResultUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };
```

Do the same for `float_union` in `build_trait_schema()` (around line 818) and the `json_value` union in `build_recursive_union_schema()` (around line 938).

**Step 2: Run all tests**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass

**Step 3: Commit**

Message: `test: update existing tests for strict-by-default unions`

---

### Task 9: Final Verification

**Step 1: Run full test suite**

Run: `cd plugins/rust && cargo test`
Expected: all tests pass

**Step 2: Run clippy**

Run: `cd plugins/rust && cargo clippy -- -D warnings`
Expected: no warnings

**Step 3: Run rustfmt**

Run: `cd plugins/rust && cargo fmt --check`
Expected: no formatting issues (run `cargo fmt` if needed)

**Step 4: Commit any formatting fixes**

Message: `style: apply rustfmt`
