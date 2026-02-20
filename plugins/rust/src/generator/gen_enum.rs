use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, EnumDef, TypeKind};

use super::naming::{type_name, variant_name};
use super::type_mapper::{enum_base_rust_type, enum_read_method, enum_write_method, fixed_size};
use super::{emit_deprecated, emit_doc_comment, LifetimeAnalysis};

/// Generate Rust code for an enum definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
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

  if is_flags {
    generate_flags(
      def, enum_def, output, &name, base_type, byte_size, is_signed, base_kind,
    )?;
  } else {
    generate_enum(
      def,
      enum_def,
      output,
      &name,
      base_type,
      read_method,
      write_method,
      byte_size,
      is_signed,
      base_kind,
    )?;
  }

  Ok(())
}

/// Generate a proper `#[repr(T)]` Rust enum for non-flags enums.
#[allow(clippy::too_many_arguments)]
fn generate_enum(
  def: &DefinitionDescriptor,
  enum_def: &EnumDef,
  output: &mut String,
  name: &str,
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

  // Enum definition with repr
  output.push_str(&format!("#[repr({})]\n", base_type));
  output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
  output.push_str(&format!("pub enum {} {{\n", name));

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

    output.push_str(&format!("  {} = {},\n", mname, formatted_value));
  }
  output.push_str("}\n\n");

  // TryFrom<base_type> for Name
  output.push_str(&format!(
    "impl core::convert::TryFrom<{}> for {} {{\n",
    base_type, name
  ));
  output.push_str("  type Error = DecodeError;\n");
  output.push_str(&format!(
    "  fn try_from(value: {}) -> Result<Self, DecodeError> {{\n",
    base_type
  ));
  output.push_str("    match value {\n");
  for m in members {
    let mname = variant_name(m.name.as_deref().unwrap_or("Unknown"));
    let mvalue = m.value.unwrap_or(0);
    let formatted_value = if is_signed {
      format_signed_value(mvalue, base_kind)
    } else {
      format!("{}", mvalue)
    };
    output.push_str(&format!(
      "      {} => Ok(Self::{}),\n",
      formatted_value, mname
    ));
  }
  output.push_str(&format!(
    "      _ => Err(DecodeError::InvalidEnum {{ type_name: \"{}\", value: value as u64 }}),\n",
    name
  ));
  output.push_str("    }\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  // From<Name> for base_type
  output.push_str(&format!("impl From<{}> for {} {{\n", name, base_type));
  output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value as {} }}\n",
    name, base_type, base_type
  ));
  output.push_str("}\n\n");

  // ── FIXED_ENCODED_SIZE ──────────────────────────────────────────
  output.push_str(&format!("impl {} {{\n", name));
  output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n",
    byte_size
  ));
  output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  // ── impl BebopEncode ──────────────────────────────────────────
  output.push_str(&format!("impl BebopEncode for {} {{\n", name));
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    name
  ));
  output.push_str(&format!(
    "    writer.{}(*self as {});\n",
    write_method, base_type
  ));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");
  output.push_str("  fn encoded_size(&self) -> usize { Self::FIXED_ENCODED_SIZE }\n");
  output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  output.push_str(&format!("impl<'buf> BebopDecode<'buf> for {} {{\n", name));
  output.push_str("  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  output.push_str(&format!("    let value = reader.{}()?;\n", read_method));
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  output.push_str("    Self::try_from(value)\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  Ok(())
}

/// Generate a newtype struct with bitflags utility methods for @flags enums.
#[allow(clippy::too_many_arguments)]
fn generate_flags(
  def: &DefinitionDescriptor,
  enum_def: &EnumDef,
  output: &mut String,
  name: &str,
  base_type: &str,
  byte_size: usize,
  is_signed: bool,
  base_kind: TypeKind,
) -> Result<(), GeneratorError> {
  let members = enum_def.members.as_deref().unwrap_or(&[]);

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Derive + struct
  output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
  output.push_str(&format!("pub struct {}(pub {});\n\n", name, base_type));

  // Associated constants
  output.push_str(&format!(
    "#[allow(non_upper_case_globals)]\nimpl {} {{\n",
    name
  ));
  output.push_str(&format!(
    "  pub const FIXED_ENCODED_SIZE: usize = {};\n",
    byte_size
  ));
  // Compute ALL value for all() method
  let mut all_value: u64 = 0;
  for m in members {
    let mname = m.name.as_deref().unwrap_or("UNKNOWN");
    let mvalue = m.value.unwrap_or(0);

    emit_doc_comment(output, &m.documentation);
    emit_deprecated(output, &m.decorators);

    let formatted_value = if is_signed {
      format_signed_value(mvalue, base_kind)
    } else {
      format!("{}", mvalue)
    };

    output.push_str(&format!(
      "  pub const {}: Self = Self({});\n",
      mname, formatted_value
    ));

    all_value |= mvalue;
  }
  output.push_str(&format!(
    "  // @@bebop_insertion_point(enum_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  // Flags trait impl
  let all_formatted = if is_signed {
    format_signed_value(all_value, base_kind)
  } else {
    format!("{}", all_value)
  };

  output.push_str(&format!("impl BebopFlags for {} {{\n", name));
  output.push_str(&format!("  type Bits = {};\n", base_type));
  output.push_str(&format!(
    "  const ALL_BITS: Self::Bits = {};\n",
    all_formatted
  ));
  output.push_str("  fn bits(self) -> Self::Bits { self.0 }\n");
  output.push_str("  fn from_bits_retain(bits: Self::Bits) -> Self { Self(bits) }\n");
  output.push_str("}\n\n");

  // Bitwise operator impls in compact single-line style.
  output.push_str(&format!(
    "impl core::ops::BitOr for {} {{ type Output = Self; fn bitor(self, rhs: Self) -> Self {{ Self(self.0 | rhs.0) }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::BitOrAssign for {} {{ fn bitor_assign(&mut self, rhs: Self) {{ self.0 |= rhs.0; }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::BitAnd for {} {{ type Output = Self; fn bitand(self, rhs: Self) -> Self {{ Self(self.0 & rhs.0) }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::BitAndAssign for {} {{ fn bitand_assign(&mut self, rhs: Self) {{ self.0 &= rhs.0; }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::BitXor for {} {{ type Output = Self; fn bitxor(self, rhs: Self) -> Self {{ Self(self.0 ^ rhs.0) }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::BitXorAssign for {} {{ fn bitxor_assign(&mut self, rhs: Self) {{ self.0 ^= rhs.0; }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::Not for {} {{ type Output = Self; fn not(self) -> Self {{ Self(!self.0) }} }}\n",
    name
  ));
  output.push_str(&format!(
    "impl core::ops::Sub for {} {{ type Output = Self; fn sub(self, rhs: Self) -> Self {{ Self(self.0 & !rhs.0) }} }}\n\n",
    name
  ));

  Ok(())
}

/// Format a u64 value for a signed enum base type.
fn format_signed_value(value: u64, kind: TypeKind) -> String {
  match kind {
    TypeKind::Int8 => format!("{}", value as u8 as i8),
    TypeKind::Int16 => format!("{}", value as u16 as i16),
    TypeKind::Int32 => format!("{}", value as u32 as i32),
    TypeKind::Int64 => format!("{}", value as i64),
    _ => format!("{}", value),
  }
}
