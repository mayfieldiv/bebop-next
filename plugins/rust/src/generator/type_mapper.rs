use crate::error::GeneratorError;
use crate::generated::{TypeDescriptor, TypeKind};

use super::naming::fqn_to_type_name;
use super::LifetimeAnalysis;

/// Map a scalar TypeKind to its Rust type string.
pub fn scalar_type(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::Bool => Some("bool"),
    TypeKind::Byte => Some("u8"),
    TypeKind::Int8 => Some("i8"),
    TypeKind::Int16 => Some("i16"),
    TypeKind::Uint16 => Some("u16"),
    TypeKind::Int32 => Some("i32"),
    TypeKind::Uint32 => Some("u32"),
    TypeKind::Int64 => Some("i64"),
    TypeKind::Uint64 => Some("u64"),
    TypeKind::Int128 => Some("i128"),
    TypeKind::Uint128 => Some("u128"),
    TypeKind::Float32 => Some("f32"),
    TypeKind::Float64 => Some("f64"),
    TypeKind::String => Some("string::String"),
    TypeKind::Float16 => Some("bebop::f16"),
    TypeKind::Bfloat16 => Some("bebop::bf16"),
    TypeKind::Uuid => Some("bebop::Uuid"),
    TypeKind::Timestamp => Some("bebop::BebopTimestamp"),
    TypeKind::Duration => Some("bebop::BebopDuration"),
    _ => None,
  }
}

/// Map a scalar TypeKind to its BebopReader read method name.
pub fn scalar_read_method(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::Bool => Some("read_bool"),
    TypeKind::Byte => Some("read_byte"),
    TypeKind::Int8 => Some("read_i8"),
    TypeKind::Int16 => Some("read_i16"),
    TypeKind::Uint16 => Some("read_u16"),
    TypeKind::Int32 => Some("read_i32"),
    TypeKind::Uint32 => Some("read_u32"),
    TypeKind::Int64 => Some("read_i64"),
    TypeKind::Uint64 => Some("read_u64"),
    TypeKind::Int128 => Some("read_i128"),
    TypeKind::Uint128 => Some("read_u128"),
    TypeKind::Float16 => Some("read_f16"),
    TypeKind::Float32 => Some("read_f32"),
    TypeKind::Float64 => Some("read_f64"),
    TypeKind::Bfloat16 => Some("read_bf16"),
    TypeKind::String => Some("read_string"),
    TypeKind::Uuid => Some("read_uuid"),
    TypeKind::Timestamp => Some("read_timestamp"),
    TypeKind::Duration => Some("read_duration"),
    _ => None,
  }
}

/// Map a scalar TypeKind to its BebopWriter write method name.
pub fn scalar_write_method(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::Bool => Some("write_bool"),
    TypeKind::Byte => Some("write_byte"),
    TypeKind::Int8 => Some("write_i8"),
    TypeKind::Int16 => Some("write_i16"),
    TypeKind::Uint16 => Some("write_u16"),
    TypeKind::Int32 => Some("write_i32"),
    TypeKind::Uint32 => Some("write_u32"),
    TypeKind::Int64 => Some("write_i64"),
    TypeKind::Uint64 => Some("write_u64"),
    TypeKind::Int128 => Some("write_i128"),
    TypeKind::Uint128 => Some("write_u128"),
    TypeKind::Float16 => Some("write_f16"),
    TypeKind::Float32 => Some("write_f32"),
    TypeKind::Float64 => Some("write_f64"),
    TypeKind::Bfloat16 => Some("write_bf16"),
    TypeKind::String => Some("write_string"),
    TypeKind::Uuid => Some("write_uuid"),
    TypeKind::Timestamp => Some("write_timestamp"),
    TypeKind::Duration => Some("write_duration"),
    _ => None,
  }
}

/// Returns true if the TypeKind is a primitive scalar with a FixedScalar impl.
fn is_fixed_scalar(kind: TypeKind) -> bool {
  matches!(
    kind,
    TypeKind::Bool
      | TypeKind::Byte
      | TypeKind::Int8
      | TypeKind::Int16
      | TypeKind::Uint16
      | TypeKind::Int32
      | TypeKind::Uint32
      | TypeKind::Int64
      | TypeKind::Uint64
      | TypeKind::Int128
      | TypeKind::Uint128
      | TypeKind::Float16
      | TypeKind::Bfloat16
      | TypeKind::Float32
      | TypeKind::Float64
  )
}

/// Returns true if the TypeKind is safe for bulk memcpy (all bit patterns valid).
/// Excludes Bool (UB for non-0/1 values) and Byte (already uses
/// write_byte_array/read_byte_slice which is even better — zero-copy on read).
pub(crate) fn is_bulk_scalar(kind: TypeKind) -> bool {
  matches!(
    kind,
    TypeKind::Int8
      | TypeKind::Int16
      | TypeKind::Uint16
      | TypeKind::Int32
      | TypeKind::Uint32
      | TypeKind::Int64
      | TypeKind::Uint64
      | TypeKind::Int128
      | TypeKind::Uint128
      | TypeKind::Float16
      | TypeKind::Bfloat16
      | TypeKind::Float32
      | TypeKind::Float64
  )
}

/// Returns true if the TypeKind is a valid enum/flags base type (integer types only).
fn is_enum_base_type(kind: TypeKind) -> bool {
  matches!(
    kind,
    TypeKind::Byte
      | TypeKind::Int8
      | TypeKind::Int16
      | TypeKind::Uint16
      | TypeKind::Int32
      | TypeKind::Uint32
      | TypeKind::Int64
      | TypeKind::Uint64
  )
}

/// Map an enum base TypeKind to its Rust integer type.
pub fn enum_base_rust_type(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  if !is_enum_base_type(kind) {
    return Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type kind: {}",
      kind as u8
    )));
  }
  scalar_type(kind).ok_or_else(|| unreachable!())
}

/// Map an enum base TypeKind to its BebopReader read method.
pub fn enum_read_method(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  if !is_enum_base_type(kind) {
    return Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type for read: {}",
      kind as u8
    )));
  }
  scalar_read_method(kind).ok_or_else(|| unreachable!())
}

/// Map an enum base TypeKind to its BebopWriter write method.
pub fn enum_write_method(kind: TypeKind) -> Result<&'static str, GeneratorError> {
  if !is_enum_base_type(kind) {
    return Err(GeneratorError::MalformedType(format!(
      "unsupported enum base type for write: {}",
      kind as u8
    )));
  }
  scalar_write_method(kind).ok_or_else(|| unreachable!())
}

/// Returns true if a scalar TypeKind needs `&` when passed to its write method.
fn scalar_needs_ref(kind: TypeKind) -> bool {
  matches!(kind, TypeKind::String)
}

/// Return the fixed byte size of a scalar type, or None for variable-size types.
pub fn fixed_size(kind: TypeKind) -> Option<usize> {
  match kind {
    TypeKind::Bool | TypeKind::Byte | TypeKind::Int8 => Some(1),
    TypeKind::Int16 | TypeKind::Uint16 | TypeKind::Float16 | TypeKind::Bfloat16 => Some(2),
    TypeKind::Int32 | TypeKind::Uint32 | TypeKind::Float32 => Some(4),
    TypeKind::Int64 | TypeKind::Uint64 | TypeKind::Float64 => Some(8),
    TypeKind::Int128 | TypeKind::Uint128 | TypeKind::Uuid => Some(16),
    TypeKind::Timestamp | TypeKind::Duration => Some(12),
    _ => None,
  }
}

/// Return a constant expression for the fixed byte size of a scalar type.
///
/// This is used in generated `encoded_size()` expressions to avoid magic numbers.
pub fn fixed_size_expr(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::Bool => Some("mem::size_of::<bool>()"),
    TypeKind::Byte => Some("mem::size_of::<u8>()"),
    TypeKind::Int8 => Some("mem::size_of::<i8>()"),
    TypeKind::Int16 => Some("mem::size_of::<i16>()"),
    TypeKind::Uint16 => Some("mem::size_of::<u16>()"),
    TypeKind::Int32 => Some("mem::size_of::<i32>()"),
    TypeKind::Uint32 => Some("mem::size_of::<u32>()"),
    TypeKind::Int64 => Some("mem::size_of::<i64>()"),
    TypeKind::Uint64 => Some("mem::size_of::<u64>()"),
    TypeKind::Int128 => Some("mem::size_of::<i128>()"),
    TypeKind::Uint128 => Some("mem::size_of::<u128>()"),
    TypeKind::Float16 => Some("mem::size_of::<bebop::f16>()"),
    TypeKind::Bfloat16 => Some("mem::size_of::<bebop::bf16>()"),
    TypeKind::Float32 => Some("mem::size_of::<f32>()"),
    TypeKind::Float64 => Some("mem::size_of::<f64>()"),
    TypeKind::Uuid => Some("mem::size_of::<bebop::Uuid>()"),
    TypeKind::Timestamp | TypeKind::Duration => {
      Some("mem::size_of::<i64>() + mem::size_of::<i32>()")
    }
    _ => None,
  }
}

/// Return a constant Rust expression for a type's fixed encoded size.
///
/// Returns `Ok(None)` for variable-size types.
pub fn fixed_encoded_size_expression(
  td: &TypeDescriptor,
) -> Result<Option<String>, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if let Some(expr) = fixed_size_expr(kind) {
    return Ok(Some(expr.to_string()));
  }

  match kind {
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let elem_expr = fixed_encoded_size_expression(elem)?;
      Ok(elem_expr.map(|e| {
        if e.contains(" + ") {
          format!("({}) * {}", e, size)
        } else {
          format!("{} * {}", e, size)
        }
      }))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      Ok(Some(format!("{}::FIXED_ENCODED_SIZE", type_name)))
    }
    _ => Ok(None),
  }
}

// ═══════════════════════════════════════════════════════════════════
// Cow-aware functions for zero-copy code generation
// ═══════════════════════════════════════════════════════════════════

/// Map a scalar TypeKind to its zero-copy Rust type string.
/// String → `Cow<'buf, str>`, others unchanged.
fn scalar_type_cow(kind: TypeKind) -> Option<&'static str> {
  match kind {
    TypeKind::String => Some("borrow::Cow<'buf, str>"),
    _ => scalar_type(kind),
  }
}

/// Returns true if a TypeDescriptor is directly a Cow-wrapped field
/// (String → Cow<str>, byte array → Cow<[u8]>), as opposed to a defined type
/// that transitively contains Cow fields.
pub fn is_cow_field(td: &TypeDescriptor) -> bool {
  let kind = match td.kind {
    Some(k) => k,
    None => return false,
  };
  match kind {
    TypeKind::String => true,
    TypeKind::Array => {
      if is_byte_array_cow_field(td) {
        return true;
      }
      td.array_element
        .as_ref()
        .is_some_and(|e| e.kind.is_some_and(is_bulk_scalar))
    }
    _ => false,
  }
}

/// Returns true when a field is represented as `BebopBytes<'buf>` (byte array).
pub fn is_byte_array_cow_field(td: &TypeDescriptor) -> bool {
  td.kind == Some(TypeKind::Array)
    && td
      .array_element
      .as_ref()
      .is_some_and(|e| e.kind == Some(TypeKind::Byte))
}

/// Map a full TypeDescriptor to its owned parameter type for `new()` constructors.
///
/// Uses `TypeName<'static>` for defined types with lifetime so constructor
/// parameter types compile correctly.
pub fn rust_type_owned(
  td: &TypeDescriptor,
  analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if let Some(s) = scalar_type(kind) {
    return Ok(s.to_string());
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        return Ok("bebop::BebopBytes<'static>".to_string());
      }
      // Bulk scalar arrays → Vec<T> (owned form of Cow<[T]>)
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("vec::Vec<{}>", ty));
      }
      let inner = rust_type_owned(elem, analysis)?;
      Ok(format!("vec::Vec<{}>", inner))
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let inner = rust_type_owned(elem, analysis)?;
      Ok(format!("[{}; {}]", inner, size))
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k = rust_type_owned(key, analysis)?;
      let v = rust_type_owned(val, analysis)?;
      Ok(format!("bebop::HashMap<{}, {}>", k, v))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      if analysis.lifetime_fqns.contains(fqn) {
        Ok(format!("{}<'static>", type_name))
      } else {
        Ok(type_name)
      }
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "unknown type kind: {}",
      kind as u8
    ))),
  }
}

/// Map a full TypeDescriptor to its Rust type string using Cow for borrowed types.
pub fn rust_type(
  td: &TypeDescriptor,
  analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if let Some(s) = scalar_type_cow(kind) {
    return Ok(s.to_string());
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte arrays → BebopBytes<'buf>
      if elem.kind == Some(TypeKind::Byte) {
        return Ok("bebop::BebopBytes<'buf>".to_string());
      }
      // Bulk scalar arrays → Cow<'buf, [T]>
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("borrow::Cow<'buf, [{}]>", ty));
      }
      let inner = rust_type(elem, analysis)?;
      Ok(format!("vec::Vec<{}>", inner))
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let inner = rust_type(elem, analysis)?;
      Ok(format!("[{}; {}]", inner, size))
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k = rust_type(key, analysis)?;
      let v = rust_type(val, analysis)?;
      Ok(format!("bebop::HashMap<{}, {}>", k, v))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      if analysis.lifetime_fqns.contains(fqn) {
        Ok(format!("{}<'buf>", type_name))
      } else {
        Ok(type_name)
      }
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "unknown type kind: {}",
      kind as u8
    ))),
  }
}

/// Generate a zero-copy read expression for a TypeDescriptor.
///
/// Returns an expression that produces `Result<T>` — callers append `?` as needed.
/// Strings are read as `Cow::Borrowed(reader.read_str()?)`, byte arrays as
/// `Cow::Borrowed(reader.read_byte_slice()?)`.
pub fn read_expression(
  td: &TypeDescriptor,
  reader: &str,
  _analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Scalars — special-case string for zero-copy
  match kind {
    TypeKind::String => {
      return Ok(format!(
        "result::Result::Ok(borrow::Cow::Borrowed({}.read_str()?))",
        reader
      ));
    }
    _ => {
      if let Some(method) = scalar_read_method(kind) {
        return Ok(format!("{}.{}()", reader, method));
      }
    }
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte array → BebopBytes::borrowed
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!(
          "result::Result::Ok(bebop::BebopBytes::borrowed({}.read_byte_slice()?))",
          reader
        ));
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        return Ok(format!("{}.read_scalar_array::<{}>()", reader, ty));
      }
      if elem_kind == TypeKind::Defined {
        let fqn = elem.defined_fqn.as_deref().unwrap_or("");
        let type_name = fqn_to_type_name(fqn);
        // For trait-based decode, use closure calling trait method
        Ok(format!(
          "{}.read_array(|_r| {}::decode(_r))",
          reader, type_name
        ))
      } else {
        let inner = read_expression(elem, "_r", _analysis)?;
        Ok(format!("{}.read_array(|_r| {})", reader, inner))
      }
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_fixed_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        Ok(format!("{}.read_fixed_array::<{}, {}>()", reader, ty, size))
      } else {
        let inner = read_expression(elem, reader, _analysis)?;
        Ok(format!(
          "{{ let mut _arr = [default::Default::default(); {}]; for _i in 0..{} {{ _arr[_i] = {}?; }} result::Result::Ok(_arr) }}",
          size, size, inner
        ))
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_expr = read_expression(key, "_r", _analysis)?;
      let v_expr = read_expression(val, "_r", _analysis)?;
      Ok(format!(
        "{}.read_map(|_r| result::Result::Ok(({}?, {}?)))",
        reader, k_expr, v_expr
      ))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      // Trait-based decode: Type::decode(reader) via BebopDecode
      Ok(format!("{}::decode({})", type_name, reader))
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate cow read for type kind: {}",
      kind as u8
    ))),
  }
}

/// Returns a direct borrowed `Cow` decode expression when applicable.
///
/// This is used by generators that want local statements like:
/// `let name = Cow::Borrowed(reader.read_str()?);`
pub fn borrowed_cow_read_expression(td: &TypeDescriptor, reader: &str) -> Option<String> {
  match td.kind? {
    TypeKind::String => Some(format!("borrow::Cow::Borrowed({}.read_str()?)", reader)),
    TypeKind::Array => {
      let elem = td.array_element.as_ref()?;
      if elem.kind == Some(TypeKind::Byte) {
        Some(format!(
          "bebop::BebopBytes::borrowed({}.read_byte_slice()?)",
          reader
        ))
      } else {
        None
      }
    }
    _ => None,
  }
}

/// Generate a write expression for a TypeDescriptor (Cow-aware).
///
/// For Cow<str> we call `writer.write_string(&v)` (Cow derefs to &str).
/// For Cow<[u8]> we call `writer.write_byte_array(&v)`.
/// For defined types we call `BebopEncode::encode(v, writer)`.
pub fn write_expression(
  td: &TypeDescriptor,
  value: &str,
  writer: &str,
  _analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Scalars — Cow<str> derefs to &str, so write_string(&v) works
  if let Some(method) = scalar_write_method(kind) {
    if scalar_needs_ref(kind) {
      return Ok(format!("{}.{}(&{})", writer, method, value));
    } else {
      return Ok(format!("{}.{}({})", writer, method, value));
    }
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // Byte array: Cow<[u8]> derefs to &[u8]
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!("{}.write_byte_array(&{})", writer, value));
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if elem_kind == TypeKind::Defined {
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| _el.encode(_w))",
          writer, value
        ))
      } else if is_bulk_scalar(elem_kind) {
        let ty = scalar_type(elem_kind).unwrap();
        Ok(format!(
          "{}.write_scalar_array::<{}>(&{})",
          writer, ty, value
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
        let inner = write_expression(elem, "_el", "_w", _analysis)?;
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| {{ {} }})",
          writer, value, inner
        ))
      }
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_fixed_scalar(elem_kind) {
        let size = td
          .fixed_array_size
          .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
        let ty = scalar_type(elem_kind).unwrap();
        Ok(format!(
          "{}.write_fixed_array::<{}, {}>(&{})",
          writer, ty, size, value
        ))
      } else {
        // iter() yields &T — dereference for scalar write methods that take T by value
        let elem_val = if scalar_write_method(elem_kind).is_some() && !scalar_needs_ref(elem_kind) {
          "*_el"
        } else {
          "_el"
        };
        let inner = write_expression(elem, elem_val, writer, _analysis)?;
        Ok(format!("for _el in {}.iter() {{ {} }}", value, inner))
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      // Map callbacks receive (&K, &V) — dereference scalars that write methods take by value
      let k_kind = key.kind.unwrap_or(TypeKind::Unknown);
      let v_kind = val.kind.unwrap_or(TypeKind::Unknown);
      let k_val = if scalar_write_method(k_kind).is_some() && !scalar_needs_ref(k_kind) {
        "*_k"
      } else {
        "_k"
      };
      let v_val = if scalar_write_method(v_kind).is_some() && !scalar_needs_ref(v_kind) {
        "*_v"
      } else {
        "_v"
      };
      let k_write = write_expression(key, k_val, "_w", _analysis)?;
      let v_write = write_expression(val, v_val, "_w", _analysis)?;
      Ok(format!(
        "{}.write_map(&{}, |_w, _k, _v| {{ {}; {}; }})",
        writer, value, k_write, v_write
      ))
    }
    TypeKind::Defined => Ok(format!("{}.encode({})", value, writer)),
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate cow write for type kind: {}",
      kind as u8
    ))),
  }
}

/// Generate an expression computing the encoded byte size of a value.
fn collection_ref(value: &str) -> String {
  if value.starts_with("self.") {
    format!("&{}", value)
  } else {
    value.to_string()
  }
}

pub fn encoded_size_expression(
  td: &TypeDescriptor,
  value: &str,
  _analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Fixed-size scalars
  if let Some(sz_expr) = fixed_size_expr(kind) {
    return Ok(sz_expr.to_string());
  }

  match kind {
    TypeKind::String => Ok(format!("bebop::wire_size::string_size({}.len())", value)),
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!(
          "bebop::wire_size::byte_array_size({}.len())",
          value
        ));
      }
      let items_ref = collection_ref(value);
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if let Some(sz_expr) = fixed_size_expr(elem_kind) {
        Ok(format!(
          "bebop::wire_size::array_size({}, |_el| ({}))",
          items_ref, sz_expr
        ))
      } else {
        let inner = encoded_size_expression(elem, "_el", _analysis)?;
        Ok(format!(
          "bebop::wire_size::array_size({}, |_el| {})",
          items_ref, inner
        ))
      }
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      let size = td
        .fixed_array_size
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if let Some(sz_expr) = fixed_size_expr(elem_kind) {
        Ok(format!("{}usize * ({})", size as usize, sz_expr))
      } else {
        let inner = encoded_size_expression(elem, "_el", _analysis)?;
        Ok(format!(
          "{}.iter().map(|_el| {}).sum::<usize>()",
          value, inner
        ))
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_size = encoded_size_expression(key, "_k", _analysis)?;
      let v_size = encoded_size_expression(val, "_v", _analysis)?;
      let map_ref = collection_ref(value);
      Ok(format!(
        "bebop::wire_size::map_size({}, |_k, _v| {} + {})",
        map_ref, k_size, v_size
      ))
    }
    TypeKind::Defined => Ok(format!("{}.encoded_size()", value)),
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate encoded_size for type kind: {}",
      kind as u8
    ))),
  }
}

/// Generate an expression that converts a value from borrowed to owned (Cow → Cow::Owned).
pub fn into_owned_expression(
  td: &TypeDescriptor,
  value: &str,
  analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  match kind {
    // Cow<str> → Cow::Owned(v.into_owned())
    TypeKind::String => Ok(format!("borrow::Cow::Owned({}.into_owned())", value)),
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      // BebopBytes → BebopBytes::into_owned()
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!("{}.into_owned()", value));
      }
      // Cow<[T]> for bulk scalars → Cow::Owned(v.into_owned())
      if elem.kind.is_some_and(is_bulk_scalar) {
        return Ok(format!("borrow::Cow::Owned({}.into_owned())", value));
      }
      // Vec of lifetime types → map into_owned
      if analysis.type_needs_lifetime(elem) {
        let inner = into_owned_expression(elem, "_e", analysis)?;
        Ok(format!(
          "{}.into_iter().map(|_e| {}).collect()",
          value, inner
        ))
      } else {
        Ok(value.to_string())
      }
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      if analysis.type_needs_lifetime(elem) {
        let inner = into_owned_expression(elem, "_e", analysis)?;
        Ok(format!("{}.map(|_e| {})", value, inner))
      } else {
        Ok(value.to_string())
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_needs = analysis.type_needs_lifetime(key);
      let v_needs = analysis.type_needs_lifetime(val);
      if k_needs || v_needs {
        let k_expr = if k_needs {
          into_owned_expression(key, "_k", analysis)?
        } else {
          "_k".to_string()
        };
        let v_expr = if v_needs {
          into_owned_expression(val, "_v", analysis)?
        } else {
          "_v".to_string()
        };
        Ok(format!(
          "{}.into_iter().map(|(_k, _v)| ({}, {})).collect()",
          value, k_expr, v_expr
        ))
      } else {
        Ok(value.to_string())
      }
    }
    TypeKind::Defined => {
      if let Some(ref fqn) = td.defined_fqn {
        if analysis.lifetime_fqns.contains(fqn.as_ref()) {
          return Ok(format!("{}.into_owned()", value));
        }
      }
      Ok(value.to_string())
    }
    // All other scalars are Copy — pass through
    _ => Ok(value.to_string()),
  }
}

/// Generate an expression that converts owned constructor input into borrowed field form.
///
/// Examples:
/// - `String` -> `Cow::Owned(String)`
/// - `Vec<String>` -> `Vec<Cow<'buf, str>>`
/// - `HashMap<String, String>` -> `HashMap<Cow<'buf, str>, Cow<'buf, str>>`
pub fn into_borrowed_expression(
  td: &TypeDescriptor,
  value: &str,
  analysis: &LifetimeAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  match kind {
    TypeKind::String => Ok(format!("borrow::Cow::Owned({})", value)),
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!(
          "<bebop::BebopBytes as convert::From<_>>::from({})",
          value
        ));
      }
      // Cow<[T]> for bulk scalars: Vec<T> → Cow::from(Vec)
      if elem.kind.is_some_and(is_bulk_scalar) {
        return Ok(format!("borrow::Cow::from({})", value));
      }
      if analysis.type_needs_lifetime(elem) {
        let inner = into_borrowed_expression(elem, "_e", analysis)?;
        if inner == "_e" {
          Ok(value.to_string())
        } else {
          Ok(format!(
            "{}.into_iter().map(|_e| {}).collect()",
            value, inner
          ))
        }
      } else {
        Ok(value.to_string())
      }
    }
    TypeKind::FixedArray => {
      let elem = td
        .fixed_array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("fixed array missing element type".into()))?;
      if analysis.type_needs_lifetime(elem) {
        let inner = into_borrowed_expression(elem, "_e", analysis)?;
        if inner == "_e" {
          Ok(value.to_string())
        } else {
          Ok(format!("{}.map(|_e| {})", value, inner))
        }
      } else {
        Ok(value.to_string())
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;
      let k_needs = analysis.type_needs_lifetime(key);
      let v_needs = analysis.type_needs_lifetime(val);
      if k_needs || v_needs {
        let k_expr = if k_needs {
          into_borrowed_expression(key, "_k", analysis)?
        } else {
          "_k".to_string()
        };
        let v_expr = if v_needs {
          into_borrowed_expression(val, "_v", analysis)?
        } else {
          "_v".to_string()
        };
        if k_expr == "_k" && v_expr == "_v" {
          Ok(value.to_string())
        } else {
          Ok(format!(
            "{}.into_iter().map(|(_k, _v)| ({}, {})).collect()",
            value, k_expr, v_expr
          ))
        }
      } else {
        Ok(value.to_string())
      }
    }
    // `T<'static>` can be used where `T<'buf>` is expected when `T` is covariant.
    TypeKind::Defined => Ok(value.to_string()),
    _ => Ok(value.to_string()),
  }
}

// ═══════════════════════════════════════════════════════════════════
// IntoIterator constructor helpers
// ═══════════════════════════════════════════════════════════════════

/// For a single element type, return (param_fragment, needs_into).
///
/// `param_fragment` is the type to use in `impl Into<param_fragment>` when
/// the element needs conversion, or just the raw type when it doesn't.
/// `needs_into` indicates whether `.into()` is required in the body.
fn element_into_info(
  td: &TypeDescriptor,
  analysis: &LifetimeAnalysis,
) -> Result<(String, bool), GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  match kind {
    // String → Cow<'buf, str>, needs Into
    TypeKind::String => Ok(("borrow::Cow<'buf, str>".to_string(), true)),
    TypeKind::Array => {
      // byte[] → BebopBytes, needs Into
      if is_byte_array_cow_field(td) {
        return Ok(("bebop::BebopBytes<'buf>".to_string(), true));
      }
      // Bulk scalar arrays → Cow<'buf, [T]>, needs Into
      if let Some(elem) = td.array_element.as_ref() {
        if elem.kind.is_some_and(is_bulk_scalar) {
          let ty = scalar_type(elem.kind.unwrap()).unwrap();
          return Ok((format!("borrow::Cow<'buf, [{}]>", ty), true));
        }
      }
      // Other arrays (e.g. Vec<DefinedType>) — use cow_type, no Into
      Ok((rust_type(td, analysis)?, false))
    }
    TypeKind::Map => {
      // Maps are not simple elements — use cow_type, no Into
      Ok((rust_type(td, analysis)?, false))
    }
    TypeKind::Defined => {
      // Defined types pass through as-is
      Ok((rust_type(td, analysis)?, false))
    }
    _ => {
      // Scalars — pass through, no Into needed
      if let Some(s) = scalar_type(kind) {
        Ok((s.to_string(), false))
      } else {
        Ok((rust_type(td, analysis)?, false))
      }
    }
  }
}

/// Return `IntoIterator`-based parameter type and body expression for a collection
/// field in a struct `new()` constructor.
///
/// Returns `Ok(Some((param_type, body_expr)))` for Array and Map fields that benefit
/// from `IntoIterator`. Returns `Ok(None)` for non-collection fields or fields already
/// handled by `is_cow_field` (which uses `impl Into<Cow<...>>`).
///
/// `param_name` is the variable name used in the body expression.
pub fn collection_into_iter(
  td: &TypeDescriptor,
  param_name: &str,
  analysis: &LifetimeAnalysis,
) -> Result<Option<(String, String)>, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Skip types already handled as Cow fields (String, byte[], bulk scalar[])
  if is_cow_field(td) {
    return Ok(None);
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;

      let (elem_type, needs_into) = element_into_info(elem, analysis)?;

      if needs_into {
        // impl IntoIterator<Item = impl Into<ElemType>>
        let param = format!(
          "impl iter::IntoIterator<Item = impl convert::Into<{}>>",
          elem_type
        );
        let body = format!("{}.into_iter().map(|_e| _e.into()).collect()", param_name);
        Ok(Some((param, body)))
      } else {
        // impl IntoIterator<Item = ElemType> — no conversion needed
        let param = format!("impl iter::IntoIterator<Item = {}>", elem_type);
        let body = format!("{}.into_iter().collect()", param_name);
        Ok(Some((param, body)))
      }
    }
    TypeKind::Map => {
      let key = td
        .map_key
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing key type".into()))?;
      let val = td
        .map_value
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("map missing value type".into()))?;

      let (key_type, key_into) = element_into_info(key, analysis)?;
      let (val_type, val_into) = element_into_info(val, analysis)?;

      // Build item type: (impl Into<K>, impl Into<V>) or (K, V) etc.
      let k_param = if key_into {
        format!("impl convert::Into<{}>", key_type)
      } else {
        key_type
      };
      let v_param = if val_into {
        format!("impl convert::Into<{}>", val_type)
      } else {
        val_type
      };
      let param = format!("impl iter::IntoIterator<Item = ({}, {})>", k_param, v_param);

      // Build body expression
      let k_expr = if key_into { "_k.into()" } else { "_k" };
      let v_expr = if val_into { "_v.into()" } else { "_v" };
      let body = if key_into || val_into {
        format!(
          "{}.into_iter().map(|(_k, _v)| ({}, {})).collect()",
          param_name, k_expr, v_expr
        )
      } else {
        format!("{}.into_iter().collect()", param_name)
      };

      Ok(Some((param, body)))
    }
    _ => Ok(None),
  }
}
