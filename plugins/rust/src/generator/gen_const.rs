use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, LiteralKind, LiteralValue, TypeDescriptor, TypeKind};

use super::naming::const_name;
use super::type_mapper::scalar_type;
use super::{emit_deprecated, emit_doc_comment, visibility_keyword, GeneratorOptions};

/// Generate Rust code for a const definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  options: &GeneratorOptions,
) -> Result<(), GeneratorError> {
  let const_def = def
    .const_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("const missing const_def".into()))?;
  let ty = const_def
    .r#type
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("const missing type".into()))?;
  let value = const_def
    .value
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("const missing value".into()))?;

  let raw_name = def.name.as_deref().unwrap_or("<unnamed>");
  let name = const_name(raw_name);
  let rust_type = const_rust_type(ty)?;
  let literal = literal_value(value, ty)?;

  let vis = visibility_keyword(def, options);

  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);
  output.push_str(&format!(
    "{} const {}: {} = {};\n\n",
    vis, name, rust_type, literal
  ));

  Ok(())
}

fn const_rust_type(td: &TypeDescriptor) -> Result<&'static str, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("const type descriptor missing kind".into()))?;

  match kind {
    TypeKind::String => Ok("&str"),
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("const array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        Ok("&[u8]")
      } else {
        Err(GeneratorError::MalformedType(
          "const arrays are only supported for byte[]".into(),
        ))
      }
    }
    _ => scalar_type(kind).ok_or_else(|| {
      GeneratorError::MalformedType(format!("unsupported const type kind: {}", kind as u8))
    }),
  }
}

fn literal_value(value: &LiteralValue, ty: &TypeDescriptor) -> Result<String, GeneratorError> {
  let literal_kind = value
    .kind
    .ok_or_else(|| GeneratorError::MalformedDefinition("const literal missing kind".into()))?;
  let type_kind = ty
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("const type descriptor missing kind".into()))?;

  match literal_kind {
    LiteralKind::Bool => value
      .bool_value
      .map(|v| {
        if v {
          "true".to_string()
        } else {
          "false".to_string()
        }
      })
      .ok_or_else(|| GeneratorError::MalformedDefinition("bool literal missing value".into())),
    LiteralKind::Int => {
      let v = value
        .int_value
        .ok_or_else(|| GeneratorError::MalformedDefinition("int literal missing value".into()))?;
      match type_kind {
        TypeKind::Float16 => Ok(format!("f16::from_f64_const({}f64)", v)),
        TypeKind::Bfloat16 => Ok(format!("bf16::from_f64_const({}f64)", v)),
        _ => Ok(format!("{}{}", v, int_suffix(type_kind)?)),
      }
    }
    LiteralKind::Float => {
      let v = value
        .float_value
        .ok_or_else(|| GeneratorError::MalformedDefinition("float literal missing value".into()))?;
      match type_kind {
        TypeKind::Float32 => Ok(float_literal(v, "f32")),
        TypeKind::Float64 => Ok(float_literal(v, "f64")),
        TypeKind::Float16 => Ok(half_literal(v, "f16")),
        TypeKind::Bfloat16 => Ok(half_literal(v, "bf16")),
        _ => Err(GeneratorError::MalformedType(
          "float literal used with non-float const type".into(),
        )),
      }
    }
    LiteralKind::String => value
      .string_value
      .as_deref()
      .map(|s| format!("\"{}\"", escape_rust_string(s)))
      .ok_or_else(|| GeneratorError::MalformedDefinition("string literal missing value".into())),
    LiteralKind::Uuid => value
      .uuid_value
      .as_ref()
      .map(|bytes| {
        format!(
          "::bebop_runtime::Uuid::from_bytes([{}])",
          format_hex_bytes(bytes.as_bytes())
        )
      })
      .ok_or_else(|| GeneratorError::MalformedDefinition("uuid literal missing value".into())),
    LiteralKind::Bytes => value
      .bytes_value
      .as_deref()
      .map(|bytes| format!("&[{}]", format_hex_bytes(bytes)))
      .ok_or_else(|| GeneratorError::MalformedDefinition("bytes literal missing value".into())),
    LiteralKind::Timestamp => value
      .timestamp_value
      .map(|v| {
        format!(
          "BebopTimestamp {{ seconds: {}i64, nanos: {}i32 }}",
          v.seconds, v.nanos
        )
      })
      .ok_or_else(|| GeneratorError::MalformedDefinition("timestamp literal missing value".into())),
    LiteralKind::Duration => value
      .duration_value
      .map(|v| {
        format!(
          "BebopDuration {{ seconds: {}i64, nanos: {}i32 }}",
          v.seconds, v.nanos
        )
      })
      .ok_or_else(|| GeneratorError::MalformedDefinition("duration literal missing value".into())),
    LiteralKind::Unknown => Err(GeneratorError::MalformedDefinition(
      "const literal has unknown kind".into(),
    )),
  }
}

fn int_suffix(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  match kind {
    TypeKind::Byte => Ok("u8"),
    TypeKind::Int8 => Ok("i8"),
    TypeKind::Int16 => Ok("i16"),
    TypeKind::Uint16 => Ok("u16"),
    TypeKind::Int32 => Ok("i32"),
    TypeKind::Uint32 => Ok("u32"),
    TypeKind::Int64 => Ok("i64"),
    TypeKind::Uint64 => Ok("u64"),
    TypeKind::Int128 => Ok("i128"),
    TypeKind::Uint128 => Ok("u128"),
    _ => Err(GeneratorError::MalformedType(
      "int literal used with non-integer const type".into(),
    )),
  }
}

fn float_literal(value: f64, suffix: &str) -> String {
  if value.is_nan() {
    return format!("{}::NAN", suffix);
  }
  if value.is_infinite() {
    if value.is_sign_negative() {
      return format!("{}::NEG_INFINITY", suffix);
    }
    return format!("{}::INFINITY", suffix);
  }
  format!("{:?}{}", value, suffix)
}

fn half_literal(value: f64, half_type: &str) -> String {
  if value.is_nan() {
    return format!("{}::NAN", half_type);
  }
  if value.is_infinite() {
    if value.is_sign_negative() {
      return format!("{}::NEG_INFINITY", half_type);
    }
    return format!("{}::INFINITY", half_type);
  }
  format!("{}::from_f64_const({:?}f64)", half_type, value)
}

fn escape_rust_string(value: &str) -> String {
  value.chars().flat_map(|c| c.escape_default()).collect()
}

fn format_hex_bytes(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|b| format!("0x{:02X}", b))
    .collect::<Vec<_>>()
    .join(", ")
}
