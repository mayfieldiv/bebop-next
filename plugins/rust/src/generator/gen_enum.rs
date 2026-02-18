use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeKind};

use super::naming::type_name;
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
    TypeKind::INT8 | TypeKind::INT16 | TypeKind::INT32 | TypeKind::INT64
  );

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Derive + struct
  output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
  output.push_str(&format!("pub struct {}(pub {});\n\n", name, base_type));

  // Associated constants
  if let Some(ref members) = enum_def.members {
    output.push_str(&format!(
      "#[allow(non_upper_case_globals)]\nimpl {} {{\n",
      name
    ));
    for m in members {
      let mname = m.name.as_deref().unwrap_or("UNKNOWN");
      let mvalue = m.value.unwrap_or(0);

      // Doc comment for member
      emit_doc_comment(output, &m.documentation);
      emit_deprecated(output, &m.decorators);

      // Format value with proper cast for signed types
      let formatted_value = if is_signed {
        format_signed_value(mvalue, base_kind)
      } else {
        format!("{}", mvalue)
      };

      output.push_str(&format!(
        "  pub const {}: Self = Self({});\n",
        mname, formatted_value
      ));
    }
    output.push_str("}\n\n");
  }

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

  // For @flags enums: BitOr, BitAnd, Not
  if is_flags {
    // BitOr
    output.push_str(&format!(
      "impl std::ops::BitOr for {} {{\n  type Output = Self;\n  fn bitor(self, rhs: Self) -> Self {{ Self(self.0 | rhs.0) }}\n}}\n\n",
      name
    ));
    // BitAnd
    output.push_str(&format!(
      "impl std::ops::BitAnd for {} {{\n  type Output = Self;\n  fn bitand(self, rhs: Self) -> Self {{ Self(self.0 & rhs.0) }}\n}}\n\n",
      name
    ));
    // Not
    output.push_str(&format!(
      "impl std::ops::Not for {} {{\n  type Output = Self;\n  fn not(self) -> Self {{ Self(!self.0) }}\n}}\n\n",
      name
    ));
  }

  Ok(())
}

/// Format a u64 value for a signed enum base type.
fn format_signed_value(value: u64, kind: TypeKind) -> String {
  match kind {
    TypeKind::INT8 => format!("{}", value as u8 as i8),
    TypeKind::INT16 => format!("{}", value as u16 as i16),
    TypeKind::INT32 => format!("{}", value as u32 as i32),
    TypeKind::INT64 => format!("{}", value as i64),
    _ => format!("{}", value),
  }
}
