use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, EnumMemberDescriptor, TypeKind};

use super::naming::{type_name, variant_name};
use super::type_mapper::{enum_base_rust_type, enum_read_method, enum_write_method, fixed_size};
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis, FORWARD_COMPATIBLE,
};

/// Shared context for enum code generation, built once in `generate()`.
struct EnumCtx<'a> {
  def: &'a DefinitionDescriptor<'a>,
  members: &'a [EnumMemberDescriptor<'a>],
  output: &'a mut String,
  options: &'a GeneratorOptions,
  name: String,
  vis: &'static str,
  base_type: &'static str,
  read_method: &'static str,
  write_method: &'static str,
  byte_size: usize,
  is_signed: bool,
  base_kind: TypeKind,
}

impl EnumCtx<'_> {
  /// Format a member's discriminator value as a Rust literal string.
  fn format_value(&self, value: u64) -> String {
    if self.is_signed {
      match self.base_kind {
        TypeKind::Int8 => format!("{}", value as u8 as i8),
        TypeKind::Int16 => format!("{}", value as u16 as i16),
        TypeKind::Int32 => format!("{}", value as u32 as i32),
        TypeKind::Int64 => format!("{}", value as i64),
        _ => format!("{}", value),
      }
    } else {
      format!("{}", value)
    }
  }
}

/// Generate Rust code for an enum definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  options: &GeneratorOptions,
  _analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));

  let enum_def = def
    .enum_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("enum missing enum_def".into()))?;

  let base_kind = enum_def
    .base_type
    .ok_or_else(|| GeneratorError::MalformedDefinition("enum missing base_type".into()))?;

  let base_type = enum_base_rust_type(base_kind)?;
  let read_method = enum_read_method(base_kind)?;
  let write_method = enum_write_method(base_kind)?;
  let byte_size = fixed_size(base_kind)
    .ok_or_else(|| GeneratorError::MalformedType("enum base type has no fixed size".into()))?;
  let is_flags = enum_def.is_flags.unwrap_or(false);
  let is_signed = matches!(
    base_kind,
    TypeKind::Int8 | TypeKind::Int16 | TypeKind::Int32 | TypeKind::Int64
  );

  let vis = visibility_keyword(def, options);
  let is_forward_compatible = has_decorator(def, FORWARD_COMPATIBLE);

  let mut ctx = EnumCtx {
    def,
    members: enum_def.members.as_deref().unwrap_or(&[]),
    output,
    options,
    name,
    vis,
    base_type,
    read_method,
    write_method,
    byte_size,
    is_signed,
    base_kind,
  };

  if is_flags {
    generate_flags(&mut ctx, is_forward_compatible)?;
  } else if is_forward_compatible {
    generate_forward_compatible_enum(&mut ctx)?;
  } else {
    generate_enum(&mut ctx)?;
  }

  Ok(())
}

/// Generate a proper `#[repr(T)]` Rust enum for non-flags enums.
fn generate_enum(ctx: &mut EnumCtx) -> Result<(), GeneratorError> {
  // Doc comment + deprecated
  emit_doc_comment(ctx.output, &ctx.def.documentation);
  emit_deprecated(ctx.output, &ctx.def.decorators);

  // Enum definition with repr
  ctx
    .output
    .push_str(&format!("#[repr({})]\n", ctx.base_type));
  ctx.options.serde.emit_derive(ctx.output);
  ctx
    .output
    .push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
  ctx
    .output
    .push_str(&format!("{} enum {} {{\n", ctx.vis, ctx.name));

  for m in ctx.members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);

    emit_doc_comment(ctx.output, &m.documentation);
    emit_deprecated(ctx.output, &m.decorators);

    let formatted_value = ctx.format_value(mvalue);
    ctx
      .output
      .push_str(&format!("  {} = {},\n", mname, formatted_value));
  }
  ctx.output.push_str("}\n\n");

  // TryFrom<base_type> for Name
  ctx.output.push_str(&format!(
    "impl ::core::convert::TryFrom<{}> for {} {{\n",
    ctx.base_type, ctx.name
  ));
  ctx.output.push_str("  type Error = DecodeError;\n");
  ctx.output.push_str(&format!(
    "  fn try_from(value: {}) -> ::core::result::Result<Self, DecodeError> {{\n",
    ctx.base_type
  ));
  ctx.output.push_str("    match value {\n");
  for m in ctx.members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = ctx.format_value(mvalue);
    ctx.output.push_str(&format!(
      "      {} => ::core::result::Result::Ok(Self::{}),\n",
      formatted_value, mname
    ));
  }
  ctx.output.push_str(&format!(
    "      _ => ::core::result::Result::Err(DecodeError::InvalidEnum {{ type_name: \"{}\", value: value as u64 }}),\n",
    ctx.name
  ));
  ctx.output.push_str("    }\n");
  ctx.output.push_str("  }\n");
  ctx.output.push_str("}\n\n");

  // From<Name> for base_type
  ctx.output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n",
    ctx.name, ctx.base_type
  ));
  ctx.output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value as {} }}\n",
    ctx.name, ctx.base_type, ctx.base_type
  ));
  ctx.output.push_str("}\n\n");

  // ── FIXED_ENCODED_SIZE ──────────────────────────────────────────
  emit_fixed_encoded_size(ctx);

  // ── impl BebopEncode ──────────────────────────────────────────
  ctx
    .output
    .push_str(&format!("impl BebopEncode for {} {{\n", ctx.name));
  ctx
    .output
    .push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "    writer.{}(*self as {});\n",
    ctx.write_method, ctx.base_type
  ));
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    ctx.name
  ));
  ctx.output.push_str("  }\n\n");
  ctx
    .output
    .push_str("  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }\n");
  ctx.output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  ctx.output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {} {{\n",
    ctx.name
  ));
  ctx.output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    ctx.name
  ));
  ctx
    .output
    .push_str(&format!("    let value = reader.{}()?;\n", ctx.read_method));
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    ctx.name
  ));
  ctx
    .output
    .push_str("    ::core::convert::TryFrom::try_from(value)\n");
  ctx.output.push_str("  }\n");
  ctx.output.push_str("}\n\n");

  Ok(())
}

/// Generate a forward-compatible enum with an Unknown(T) variant.
/// Used when @forward_compatible decorator is present.
fn generate_forward_compatible_enum(ctx: &mut EnumCtx) -> Result<(), GeneratorError> {
  // Doc comment + deprecated
  emit_doc_comment(ctx.output, &ctx.def.documentation);
  emit_deprecated(ctx.output, &ctx.def.decorators);

  // Enum definition — no #[repr]
  ctx.options.serde.emit_derive(ctx.output);
  ctx
    .output
    .push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
  ctx
    .output
    .push_str(&format!("{} enum {} {{\n", ctx.vis, ctx.name));

  for m in ctx.members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    emit_doc_comment(ctx.output, &m.documentation);
    emit_deprecated(ctx.output, &m.decorators);
    ctx.output.push_str(&format!("  {},\n", mname));
  }
  ctx
    .output
    .push_str("  /// A discriminator value not recognized by this version of the schema.\n");
  ctx
    .output
    .push_str(&format!("  Unknown({}),\n", ctx.base_type));
  ctx.output.push_str("}\n\n");

  // ── impl block: FIXED_ENCODED_SIZE, discriminator(), is_known() ──
  ctx.output.push_str(&format!("impl {} {{\n", ctx.name));
  ctx.output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n\n",
    ctx.byte_size
  ));

  // discriminator()
  ctx.output.push_str(&format!(
    "  /// Returns the raw discriminator value.\n  pub fn discriminator(self) -> {} {{\n    match self {{\n",
    ctx.base_type
  ));
  for m in ctx.members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = ctx.format_value(mvalue);
    ctx
      .output
      .push_str(&format!("      Self::{} => {},\n", mname, formatted_value));
  }
  ctx.output.push_str("      Self::Unknown(v) => v,\n");
  ctx.output.push_str("    }\n  }\n\n");

  // is_known()
  ctx
    .output
    .push_str("  /// Returns `true` if this value matches a known variant.\n");
  ctx.output.push_str("  pub fn is_known(&self) -> bool {\n");
  ctx
    .output
    .push_str("    !::core::matches!(self, Self::Unknown(_))\n");
  ctx.output.push_str("  }\n");

  ctx.output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    ctx.name
  ));
  ctx.output.push_str("}\n\n");

  // ── From<base_type> for Name (infallible) ──
  ctx.output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n",
    ctx.base_type, ctx.name
  ));
  ctx.output.push_str(&format!(
    "  fn from(value: {}) -> Self {{\n    match value {{\n",
    ctx.base_type
  ));
  for m in ctx.members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = ctx.format_value(mvalue);
    ctx
      .output
      .push_str(&format!("      {} => Self::{},\n", formatted_value, mname));
  }
  ctx.output.push_str("      v => Self::Unknown(v),\n");
  ctx.output.push_str("    }\n  }\n}\n\n");

  // ── From<Name> for base_type ──
  ctx.output.push_str(&format!(
    "impl ::core::convert::From<{}> for {} {{\n",
    ctx.name, ctx.base_type
  ));
  ctx.output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value.discriminator() }}\n",
    ctx.name, ctx.base_type
  ));
  ctx.output.push_str("}\n\n");

  // ── BebopEncode ──
  ctx
    .output
    .push_str(&format!("impl BebopEncode for {} {{\n", ctx.name));
  ctx
    .output
    .push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "    writer.{}(self.discriminator());\n",
    ctx.write_method
  ));
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    ctx.name
  ));
  ctx.output.push_str("  }\n\n");
  ctx
    .output
    .push_str("  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }\n");
  ctx.output.push_str("}\n\n");

  // ── BebopDecode ──
  ctx.output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {} {{\n",
    ctx.name
  ));
  ctx.output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    ctx.name
  ));
  ctx
    .output
    .push_str(&format!("    let value = reader.{}()?;\n", ctx.read_method));
  ctx.output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    ctx.name
  ));
  ctx
    .output
    .push_str("    ::core::result::Result::Ok(<Self as ::core::convert::From<_>>::from(value))\n");
  ctx.output.push_str("  }\n");
  ctx.output.push_str("}\n\n");

  Ok(())
}

/// Generate a newtype struct with bitflags utility methods for @flags enums.
fn generate_flags(ctx: &mut EnumCtx, is_forward_compatible: bool) -> Result<(), GeneratorError> {
  // Doc comment + deprecated
  emit_doc_comment(ctx.output, &ctx.def.documentation);
  emit_deprecated(ctx.output, &ctx.def.decorators);

  // Derive + struct
  ctx.options.serde.emit_derive(ctx.output);
  ctx
    .output
    .push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
  ctx.output.push_str(&format!(
    "{} struct {}({} {});\n\n",
    ctx.vis, ctx.name, ctx.vis, ctx.base_type
  ));

  // Associated constants
  ctx.output.push_str(&format!(
    "#[allow(non_upper_case_globals)]\nimpl {} {{\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n",
    ctx.byte_size
  ));
  // Compute ALL value for all() method
  let mut all_value: u64 = 0;
  for m in ctx.members {
    let mname = m.name.as_deref().unwrap_or("UNKNOWN");
    let mvalue = m.value.unwrap_or(0);

    emit_doc_comment(ctx.output, &m.documentation);
    emit_deprecated(ctx.output, &m.decorators);

    let formatted_value = ctx.format_value(mvalue);
    ctx.output.push_str(&format!(
      "  pub const {}: Self = Self({});\n",
      mname, formatted_value
    ));

    all_value |= mvalue;
  }
  ctx.output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    ctx.name
  ));
  ctx.output.push_str("}\n\n");

  // Flags trait impl
  let all_formatted = ctx.format_value(all_value);

  ctx
    .output
    .push_str(&format!("impl BebopFlags for {} {{\n", ctx.name));
  ctx
    .output
    .push_str(&format!("  type Bits = {};\n", ctx.base_type));
  ctx.output.push_str(&format!(
    "  const ALL_BITS: Self::Bits = {};\n",
    all_formatted
  ));
  ctx
    .output
    .push_str("  fn bits(self) -> Self::Bits { self.0 }\n");
  ctx
    .output
    .push_str("  fn from_bits_retain(bits: Self::Bits) -> Self { Self(bits) }\n");
  ctx.output.push_str("}\n\n");

  // Bitwise operator impls in compact single-line style.
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitOr for {} {{ type Output = Self; fn bitor(self, rhs: Self) -> Self {{ Self(self.0 | rhs.0) }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitOrAssign for {} {{ fn bitor_assign(&mut self, rhs: Self) {{ self.0 |= rhs.0; }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitAnd for {} {{ type Output = Self; fn bitand(self, rhs: Self) -> Self {{ Self(self.0 & rhs.0) }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitAndAssign for {} {{ fn bitand_assign(&mut self, rhs: Self) {{ self.0 &= rhs.0; }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitXor for {} {{ type Output = Self; fn bitxor(self, rhs: Self) -> Self {{ Self(self.0 ^ rhs.0) }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::BitXorAssign for {} {{ fn bitxor_assign(&mut self, rhs: Self) {{ self.0 ^= rhs.0; }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::Not for {} {{ type Output = Self; fn not(self) -> Self {{ Self(!self.0) }} }}\n",
    ctx.name
  ));
  ctx.output.push_str(&format!(
    "impl ::core::ops::Sub for {} {{ type Output = Self; fn sub(self, rhs: Self) -> Self {{ Self(self.0 & !rhs.0) }} }}\n\n",
    ctx.name
  ));

  // ── BebopDecode (generated per-type, not blanket) ──
  ctx.output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {} {{\n",
    ctx.name
  ));
  ctx.output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> ::core::result::Result<Self, DecodeError> {\n",
  );
  ctx
    .output
    .push_str(&format!("    let bits = reader.{}()?;\n", ctx.read_method));
  if is_forward_compatible {
    ctx
      .output
      .push_str("    ::core::result::Result::Ok(Self::from_bits_retain(bits))\n");
  } else {
    ctx.output.push_str(&format!(
      "    Self::from_bits(bits).ok_or(DecodeError::InvalidFlags {{ type_name: \"{}\", bits: bits as u64 }})\n",
      ctx.name
    ));
  }
  ctx.output.push_str("  }\n");
  ctx.output.push_str("}\n\n");

  Ok(())
}

/// Emit `FIXED_ENCODED_SIZE` constant and insertion point for non-flags enums.
fn emit_fixed_encoded_size(ctx: &mut EnumCtx) {
  ctx.output.push_str(&format!("impl {} {{\n", ctx.name));
  ctx.output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n",
    ctx.byte_size
  ));
  ctx.output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    ctx.name
  ));
  ctx.output.push_str("}\n\n");
}
