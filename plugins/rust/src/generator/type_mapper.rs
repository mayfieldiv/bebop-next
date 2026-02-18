use crate::descriptor::{TypeDescriptor, TypeKind};
use crate::error::GeneratorError;

use super::naming::fqn_to_type_name;

/// Map a scalar TypeKind to its Rust type string.
pub fn scalar_type(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::BOOL => Some("bool"),
    TypeKind::BYTE => Some("u8"),
    TypeKind::INT8 => Some("i8"),
    TypeKind::INT16 => Some("i16"),
    TypeKind::UINT16 => Some("u16"),
    TypeKind::INT32 => Some("i32"),
    TypeKind::UINT32 => Some("u32"),
    TypeKind::INT64 => Some("i64"),
    TypeKind::UINT64 => Some("u64"),
    TypeKind::INT128 => Some("i128"),
    TypeKind::UINT128 => Some("u128"),
    TypeKind::FLOAT32 => Some("f32"),
    TypeKind::FLOAT64 => Some("f64"),
    TypeKind::STRING => Some("String"),
    TypeKind::FLOAT16 => Some("u16 /* TODO: f16 */"),
    TypeKind::BFLOAT16 => Some("u16 /* TODO: bf16 */"),
    TypeKind::UUID => Some("[u8; 16]"),
    TypeKind::TIMESTAMP => Some("(i64, i32)"),
    TypeKind::DURATION => Some("(i64, i32)"),
    _ => None,
  }
}

/// Map a scalar TypeKind to its BebopReader read method name.
pub fn scalar_read_method(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::BOOL => Some("read_bool"),
    TypeKind::BYTE => Some("read_byte"),
    TypeKind::INT8 => Some("read_i8"),
    TypeKind::INT16 => Some("read_i16"),
    TypeKind::UINT16 => Some("read_u16"),
    TypeKind::INT32 => Some("read_i32"),
    TypeKind::UINT32 => Some("read_u32"),
    TypeKind::INT64 => Some("read_i64"),
    TypeKind::UINT64 => Some("read_u64"),
    TypeKind::FLOAT32 => Some("read_f32"),
    TypeKind::FLOAT64 => Some("read_f64"),
    TypeKind::STRING => Some("read_string"),
    TypeKind::UUID => Some("read_uuid"),
    TypeKind::TIMESTAMP => Some("read_timestamp"),
    TypeKind::DURATION => Some("read_duration"),
    _ => None,
  }
}

/// Map a scalar TypeKind to its BebopWriter write method name.
pub fn scalar_write_method(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::BOOL => Some("write_bool"),
    TypeKind::BYTE => Some("write_byte"),
    TypeKind::INT8 => Some("write_i8"),
    TypeKind::INT16 => Some("write_i16"),
    TypeKind::UINT16 => Some("write_u16"),
    TypeKind::INT32 => Some("write_i32"),
    TypeKind::UINT32 => Some("write_u32"),
    TypeKind::INT64 => Some("write_i64"),
    TypeKind::UINT64 => Some("write_u64"),
    TypeKind::FLOAT32 => Some("write_f32"),
    TypeKind::FLOAT64 => Some("write_f64"),
    TypeKind::STRING => Some("write_string"),
    TypeKind::UUID => Some("write_uuid"),
    TypeKind::TIMESTAMP => Some("write_timestamp"),
    TypeKind::DURATION => Some("write_duration"),
    _ => None,
  }
}

/// Map an enum base TypeKind to its Rust integer type.
pub fn enum_base_rust_type(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  match kind {
    TypeKind::BYTE => Ok("u8"),
    TypeKind::INT16 => Ok("i16"),
    TypeKind::UINT16 => Ok("u16"),
    TypeKind::INT32 => Ok("i32"),
    TypeKind::UINT32 => Ok("u32"),
    TypeKind::INT64 => Ok("i64"),
    TypeKind::UINT64 => Ok("u64"),
    _ => Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type kind: {}",
      kind.0
    ))),
  }
}

/// Map an enum base TypeKind to its BebopReader read method.
pub fn enum_read_method(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  match kind {
    TypeKind::BYTE => Ok("read_byte"),
    TypeKind::INT16 => Ok("read_i16"),
    TypeKind::UINT16 => Ok("read_u16"),
    TypeKind::INT32 => Ok("read_i32"),
    TypeKind::UINT32 => Ok("read_u32"),
    TypeKind::INT64 => Ok("read_i64"),
    TypeKind::UINT64 => Ok("read_u64"),
    _ => Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type for read: {}",
      kind.0
    ))),
  }
}

/// Map an enum base TypeKind to its BebopWriter write method.
pub fn enum_write_method(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  match kind {
    TypeKind::BYTE => Ok("write_byte"),
    TypeKind::INT16 => Ok("write_i16"),
    TypeKind::UINT16 => Ok("write_u16"),
    TypeKind::INT32 => Ok("write_i32"),
    TypeKind::UINT32 => Ok("write_u32"),
    TypeKind::INT64 => Ok("write_i64"),
    TypeKind::UINT64 => Ok("write_u64"),
    _ => Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type for write: {}",
      kind.0
    ))),
  }
}

/// Map a full TypeDescriptor to its Rust type string.
pub fn rust_type(td: &TypeDescriptor) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if let Some(s) = scalar_type(kind) {
    return Ok(s.to_string());
  }

  match kind {
    TypeKind::ARRAY => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte arrays get special treatment
      if elem.kind == Some(TypeKind::BYTE) {
        return Ok("Vec<u8>".to_string());
      }
      let inner = rust_type(elem)?;
      Ok(format!("Vec<{}>", inner))
    }
    TypeKind::FIXED_ARRAY => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let inner = rust_type(elem)?;
      Ok(format!("[{}; {}]", inner, size))
    }
    TypeKind::MAP => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k = rust_type(key)?;
      let v = rust_type(val)?;
      Ok(format!("std::collections::HashMap<{}, {}>", k, v))
    }
    TypeKind::DEFINED => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      Ok(fqn_to_type_name(fqn))
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "unknown type kind: {}",
      kind.0
    ))),
  }
}

/// Generate a read expression for a TypeDescriptor.
///
/// Returns an expression that produces `Result<T>` — callers append `?` as needed.
pub fn read_expression(td: &TypeDescriptor, reader: &str) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Scalars
  if let Some(method) = scalar_read_method(kind) {
    return Ok(format!("{}.{}()", reader, method));
  }

  match kind {
    TypeKind::ARRAY => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte array optimization
      if elem.kind == Some(TypeKind::BYTE) {
        return Ok(format!("{}.read_byte_array()", reader));
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::UNKNOWN);
      if elem_kind == TypeKind::DEFINED {
        // DEFINED elements: use fn pointer
        let fqn = elem.defined_fqn.as_deref().unwrap_or("");
        let type_name = fqn_to_type_name(fqn);
        Ok(format!("{}.read_array({}::decode)", reader, type_name))
      } else {
        // Scalar / compound elements: use closure
        let inner = read_expression(elem, "_r")?;
        Ok(format!("{}.read_array(|_r| {})", reader, inner))
      }
    }
    TypeKind::FIXED_ARRAY => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let elem_kind = elem.kind.unwrap_or(TypeKind::UNKNOWN);
      if elem_kind == TypeKind::INT32 {
        Ok(format!("{}.read_fixed_i32_array::<{}>()", reader, size))
      } else {
        // General fixed array: read N elements
        let inner = read_expression(elem, reader)?;
        Ok(format!(
          "{{ let mut _arr = [Default::default(); {}]; for _i in 0..{} {{ _arr[_i] = {}?; }} Ok(_arr) }}",
          size, size, inner
        ))
      }
    }
    TypeKind::MAP => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_expr = read_expression(key, "_r")?;
      let v_expr = read_expression(val, "_r")?;
      Ok(format!(
        "{}.read_map(|_r| Ok(({}?, {}?)))",
        reader, k_expr, v_expr
      ))
    }
    TypeKind::DEFINED => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      Ok(format!("{}::decode({})", type_name, reader))
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate read for type kind: {}",
      kind.0
    ))),
  }
}

/// Returns true if a scalar TypeKind needs `&` when passed to its write method.
fn scalar_needs_ref(kind: TypeKind) -> bool {
  matches!(kind, TypeKind::STRING | TypeKind::UUID)
}

/// Generate a write statement for a TypeDescriptor.
pub fn write_expression(
  td: &TypeDescriptor,
  value: &str,
  writer: &str,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Scalars
  if let Some(method) = scalar_write_method(kind) {
    if scalar_needs_ref(kind) {
      return Ok(format!("{}.{}(&{})", writer, method, value));
    } else {
      return Ok(format!("{}.{}({})", writer, method, value));
    }
  }

  match kind {
    TypeKind::ARRAY => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte array optimization
      if elem.kind == Some(TypeKind::BYTE) {
        return Ok(format!("{}.write_byte_array(&{})", writer, value));
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::UNKNOWN);
      if elem_kind == TypeKind::DEFINED {
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| _el.encode(_w))",
          writer, value
        ))
      } else if let Some(method) = scalar_write_method(elem_kind) {
        if scalar_needs_ref(elem_kind) {
          Ok(format!(
            "{}.write_array(&{}, |_w, _el| _w.{}(_el))",
            writer, value, method
          ))
        } else {
          Ok(format!(
            "{}.write_array(&{}, |_w, _el| _w.{}(*_el))",
            writer, value, method
          ))
        }
      } else {
        // Compound element type
        let inner = write_expression(elem, "_el", "_w")?;
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| {{ {} }})",
          writer, value, inner
        ))
      }
    }
    TypeKind::FIXED_ARRAY => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let elem_kind = elem.kind.unwrap_or(TypeKind::UNKNOWN);
      if elem_kind == TypeKind::INT32 {
        let size = td
          .fixed_array_size
          .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
        Ok(format!(
          "{}.write_fixed_i32_array::<{}>(&{})",
          writer, size, value
        ))
      } else {
        // General: write each element
        let inner = write_expression(elem, "_el", writer)?;
        Ok(format!("for _el in {}.iter() {{ {} }}", value, inner))
      }
    }
    TypeKind::MAP => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_write = write_expression(key, "_k", "_w")?;
      let v_write = write_expression(val, "_v", "_w")?;
      Ok(format!(
        "{}.write_map(&{}, |_w, _k, _v| {{ {}; {}; }})",
        writer, value, k_write, v_write
      ))
    }
    TypeKind::DEFINED => Ok(format!("{}.encode({})", value, writer)),
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate write for type kind: {}",
      kind.0
    ))),
  }
}

/// Return the fixed byte size of a scalar type, or None for variable-size types.
pub fn fixed_size(kind: TypeKind) -> Option<usize> {
  match kind {
    TypeKind::BOOL | TypeKind::BYTE | TypeKind::INT8 => Some(1),
    TypeKind::INT16 | TypeKind::UINT16 | TypeKind::FLOAT16 | TypeKind::BFLOAT16 => Some(2),
    TypeKind::INT32 | TypeKind::UINT32 | TypeKind::FLOAT32 => Some(4),
    TypeKind::INT64 | TypeKind::UINT64 | TypeKind::FLOAT64 => Some(8),
    TypeKind::INT128 | TypeKind::UINT128 | TypeKind::UUID => Some(16),
    TypeKind::TIMESTAMP | TypeKind::DURATION => Some(12),
    _ => None,
  }
}
