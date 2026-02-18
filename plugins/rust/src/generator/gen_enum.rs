use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeKind};

use super::naming::{type_name, variant_name};
use super::type_mapper::{enum_base_rust_type, enum_read_method, enum_write_method, fixed_size};
use super::{emit_deprecated, emit_doc_comment};

/// Generate Rust code for an enum definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
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
    generate_flags(def, output, &name, base_type, read_method, write_method, byte_size, is_signed, base_kind)?;
  } else {
    generate_enum(def, output, &name, base_type, read_method, write_method, byte_size, is_signed, base_kind)?;
  }

  Ok(())
}

/// Generate a proper `#[repr(T)]` Rust enum for non-flags enums.
fn generate_enum(
  def: &DefinitionDescriptor,
  output: &mut String,
  name: &str,
  base_type: &str,
  read_method: &str,
  write_method: &str,
  byte_size: usize,
  is_signed: bool,
  base_kind: TypeKind,
) -> Result<(), GeneratorError> {
  let enum_def = def.enum_def.as_ref().unwrap();
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
    "impl std::convert::TryFrom<{}> for {} {{\n",
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
  output.push_str(&format!(
    "impl From<{}> for {} {{\n",
    name, base_type
  ));
  output.push_str(&format!(
    "  fn from(value: {}) -> {} {{ value as {} }}\n",
    name, base_type, base_type
  ));
  output.push_str("}\n\n");

  // decode/encode impl
  output.push_str(&format!("impl {} {{\n", name));
  output.push_str(
    "  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {\n",
  );
  output.push_str(&format!(
    "    Self::try_from(reader.{}()?)\n",
    read_method
  ));
  output.push_str("  }\n\n");

  output.push_str("  pub fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!(
    "    writer.{}(*self as {});\n",
    write_method, base_type
  ));
  output.push_str("  }\n\n");

  output.push_str(&format!(
    "  pub fn encoded_size(&self) -> usize {{ {} }}\n",
    byte_size
  ));
  output.push_str("}\n\n");

  Ok(())
}

/// Generate a newtype struct with bitflags utility methods for @flags enums.
fn generate_flags(
  def: &DefinitionDescriptor,
  output: &mut String,
  name: &str,
  base_type: &str,
  read_method: &str,
  write_method: &str,
  byte_size: usize,
  is_signed: bool,
  base_kind: TypeKind,
) -> Result<(), GeneratorError> {
  let enum_def = def.enum_def.as_ref().unwrap();
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
  output.push_str("}\n\n");

  // Utility methods
  let all_formatted = if is_signed {
    format_signed_value(all_value, base_kind)
  } else {
    format!("{}", all_value)
  };

  output.push_str(&format!("impl {} {{\n", name));
  output.push_str(&format!(
    "  pub fn empty() -> Self {{ Self(0) }}\n"
  ));
  output.push_str(&format!(
    "  pub fn all() -> Self {{ Self({}) }}\n",
    all_formatted
  ));
  output.push_str(&format!(
    "  pub fn bits(self) -> {} {{ self.0 }}\n",
    base_type
  ));
  output.push_str(&format!(
    "  pub fn from_bits(bits: {}) -> Option<Self> {{\n    if bits & !Self::all().0 == 0 {{ Some(Self(bits)) }} else {{ None }}\n  }}\n",
    base_type
  ));
  output.push_str(&format!(
    "  pub fn from_bits_truncate(bits: {}) -> Self {{ Self(bits & Self::all().0) }}\n",
    base_type
  ));
  output.push_str("  pub fn is_empty(self) -> bool { self.0 == 0 }\n");
  output.push_str(&format!(
    "  pub fn is_all(self) -> bool {{ self.0 == Self::all().0 }}\n"
  ));
  output.push_str("  pub fn contains(self, other: Self) -> bool { (self.0 & other.0) == other.0 }\n");
  output.push_str("  pub fn intersects(self, other: Self) -> bool { (self.0 & other.0) != 0 }\n");
  output.push_str("  pub fn insert(&mut self, other: Self) { self.0 |= other.0; }\n");
  output.push_str("  pub fn remove(&mut self, other: Self) { self.0 &= !other.0; }\n");
  output.push_str("  pub fn toggle(&mut self, other: Self) { self.0 ^= other.0; }\n");
  output.push_str("}\n\n");

  // decode/encode impl
  output.push_str(&format!("impl {} {{\n", name));
  output.push_str(&format!(
    "  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {{\n"
  ));
  output.push_str(&format!("    Ok(Self(reader.{}()?))\n", read_method));
  output.push_str("  }\n\n");

  output.push_str("  pub fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!("    writer.{}(self.0);\n", write_method));
  output.push_str("  }\n\n");

  output.push_str(&format!(
    "  pub fn encoded_size(&self) -> usize {{ {} }}\n",
    byte_size
  ));
  output.push_str("}\n\n");

  // Bitwise operator impls
  // BitOr
  output.push_str(&format!(
    "impl std::ops::BitOr for {} {{\n  type Output = Self;\n  fn bitor(self, rhs: Self) -> Self {{ Self(self.0 | rhs.0) }}\n}}\n\n",
    name
  ));
  // BitOrAssign
  output.push_str(&format!(
    "impl std::ops::BitOrAssign for {} {{\n  fn bitor_assign(&mut self, rhs: Self) {{ self.0 |= rhs.0; }}\n}}\n\n",
    name
  ));
  // BitAnd
  output.push_str(&format!(
    "impl std::ops::BitAnd for {} {{\n  type Output = Self;\n  fn bitand(self, rhs: Self) -> Self {{ Self(self.0 & rhs.0) }}\n}}\n\n",
    name
  ));
  // BitAndAssign
  output.push_str(&format!(
    "impl std::ops::BitAndAssign for {} {{\n  fn bitand_assign(&mut self, rhs: Self) {{ self.0 &= rhs.0; }}\n}}\n\n",
    name
  ));
  // BitXor
  output.push_str(&format!(
    "impl std::ops::BitXor for {} {{\n  type Output = Self;\n  fn bitxor(self, rhs: Self) -> Self {{ Self(self.0 ^ rhs.0) }}\n}}\n\n",
    name
  ));
  // BitXorAssign
  output.push_str(&format!(
    "impl std::ops::BitXorAssign for {} {{\n  fn bitxor_assign(&mut self, rhs: Self) {{ self.0 ^= rhs.0; }}\n}}\n\n",
    name
  ));
  // Not
  output.push_str(&format!(
    "impl std::ops::Not for {} {{\n  type Output = Self;\n  fn not(self) -> Self {{ Self(!self.0) }}\n}}\n\n",
    name
  ));
  // Sub (set difference)
  output.push_str(&format!(
    "impl std::ops::Sub for {} {{\n  type Output = Self;\n  fn sub(self, rhs: Self) -> Self {{ Self(self.0 & !rhs.0) }}\n}}\n\n",
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
