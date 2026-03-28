use std::mem;

use crate::error::GeneratorError;
use crate::generated::{TypeDescriptor, TypeKind};

use super::naming::fqn_to_type_name;
use super::SchemaAnalysis;

// ═══════════════════════════════════════════════════════════════════
// Fixed Scalar Table
// ═══════════════════════════════════════════════════════════════════

/// Metadata for a fixed-size scalar type. All 18 leaf `TypeKind` variants
/// except `String` share this uniform pattern: one reader method, one writer
/// method, a compile-time-known size, Copy semantics, no Cow wrapping.
pub struct FixedScalarInfo {
  pub kind: TypeKind,
  pub rust_type: &'static str,
  pub read_method: &'static str,
  pub write_method: &'static str,
  pub wire_size: usize,
  pub wire_size_expr: &'static str,
  /// Safe for `Cow<[T]>` zero-copy arrays. Excludes `Bool` (UB for non-0/1
  /// values) and `Byte` (uses `write_byte_array`/`read_byte_slice` instead).
  pub is_bulk_eligible: bool,
  /// Valid as enum/flags base type (integer types only).
  pub is_enum_base: bool,
  /// Implements the runtime `FixedScalar` trait, enabling `read_fixed_array`
  /// / `write_fixed_array`. True for the 15 primitive numeric types (Bool
  /// through Float64); false for Uuid, Timestamp, Duration.
  pub is_fixed_array_scalar: bool,
}

pub(super) static FIXED_SCALAR_TABLE: &[FixedScalarInfo] = &[
  FixedScalarInfo {
    kind: TypeKind::Bool,
    rust_type: "bool",
    read_method: "read_bool",
    write_method: "write_bool",
    wire_size: mem::size_of::<bool>(),
    wire_size_expr: "mem::size_of::<bool>()",
    is_bulk_eligible: false,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Byte,
    rust_type: "u8",
    read_method: "read_byte",
    write_method: "write_byte",
    wire_size: mem::size_of::<u8>(),
    wire_size_expr: "mem::size_of::<u8>()",
    is_bulk_eligible: false,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Int8,
    rust_type: "i8",
    read_method: "read_i8",
    write_method: "write_i8",
    wire_size: mem::size_of::<i8>(),
    wire_size_expr: "mem::size_of::<i8>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Int16,
    rust_type: "i16",
    read_method: "read_i16",
    write_method: "write_i16",
    wire_size: mem::size_of::<i16>(),
    wire_size_expr: "mem::size_of::<i16>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Uint16,
    rust_type: "u16",
    read_method: "read_u16",
    write_method: "write_u16",
    wire_size: mem::size_of::<u16>(),
    wire_size_expr: "mem::size_of::<u16>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Int32,
    rust_type: "i32",
    read_method: "read_i32",
    write_method: "write_i32",
    wire_size: mem::size_of::<i32>(),
    wire_size_expr: "mem::size_of::<i32>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Uint32,
    rust_type: "u32",
    read_method: "read_u32",
    write_method: "write_u32",
    wire_size: mem::size_of::<u32>(),
    wire_size_expr: "mem::size_of::<u32>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Int64,
    rust_type: "i64",
    read_method: "read_i64",
    write_method: "write_i64",
    wire_size: mem::size_of::<i64>(),
    wire_size_expr: "mem::size_of::<i64>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Uint64,
    rust_type: "u64",
    read_method: "read_u64",
    write_method: "write_u64",
    wire_size: mem::size_of::<u64>(),
    wire_size_expr: "mem::size_of::<u64>()",
    is_bulk_eligible: true,
    is_enum_base: true,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Int128,
    rust_type: "i128",
    read_method: "read_i128",
    write_method: "write_i128",
    wire_size: mem::size_of::<i128>(),
    wire_size_expr: "mem::size_of::<i128>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Uint128,
    rust_type: "u128",
    read_method: "read_u128",
    write_method: "write_u128",
    wire_size: mem::size_of::<u128>(),
    wire_size_expr: "mem::size_of::<u128>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Float16,
    rust_type: "bebop::f16",
    read_method: "read_f16",
    write_method: "write_f16",
    wire_size: mem::size_of::<bebop_runtime::f16>(),
    wire_size_expr: "mem::size_of::<bebop::f16>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Bfloat16,
    rust_type: "bebop::bf16",
    read_method: "read_bf16",
    write_method: "write_bf16",
    wire_size: mem::size_of::<bebop_runtime::bf16>(),
    wire_size_expr: "mem::size_of::<bebop::bf16>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Float32,
    rust_type: "f32",
    read_method: "read_f32",
    write_method: "write_f32",
    wire_size: mem::size_of::<f32>(),
    wire_size_expr: "mem::size_of::<f32>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Float64,
    rust_type: "f64",
    read_method: "read_f64",
    write_method: "write_f64",
    wire_size: mem::size_of::<f64>(),
    wire_size_expr: "mem::size_of::<f64>()",
    is_bulk_eligible: true,
    is_enum_base: false,
    is_fixed_array_scalar: true,
  },
  FixedScalarInfo {
    kind: TypeKind::Uuid,
    rust_type: "bebop::Uuid",
    read_method: "read_uuid",
    write_method: "write_uuid",
    wire_size: mem::size_of::<bebop_runtime::Uuid>(),
    wire_size_expr: "mem::size_of::<bebop::Uuid>()",
    is_bulk_eligible: false,
    is_enum_base: false,
    is_fixed_array_scalar: false,
  },
  FixedScalarInfo {
    kind: TypeKind::Timestamp,
    rust_type: "bebop::BebopTimestamp",
    read_method: "read_timestamp",
    write_method: "write_timestamp",
    // Wire size is i64 + i32 (no padding), not size_of the struct
    wire_size: mem::size_of::<i64>() + mem::size_of::<i32>(),
    wire_size_expr: "mem::size_of::<i64>() + mem::size_of::<i32>()",
    is_bulk_eligible: false,
    is_enum_base: false,
    is_fixed_array_scalar: false,
  },
  FixedScalarInfo {
    kind: TypeKind::Duration,
    rust_type: "bebop::BebopDuration",
    read_method: "read_duration",
    write_method: "write_duration",
    // Wire size is i64 + i32 (no padding), not size_of the struct
    wire_size: mem::size_of::<i64>() + mem::size_of::<i32>(),
    wire_size_expr: "mem::size_of::<i64>() + mem::size_of::<i32>()",
    is_bulk_eligible: false,
    is_enum_base: false,
    is_fixed_array_scalar: false,
  },
];

/// Look up the fixed scalar info for a `TypeKind`.
/// Returns `None` for `String`, compound types, and `Unknown`.
pub fn fixed_scalar_info(kind: TypeKind) -> Option<&'static FixedScalarInfo> {
  FIXED_SCALAR_TABLE.iter().find(|s| s.kind == kind)
}

// ═══════════════════════════════════════════════════════════════════
// FieldCodegen
// ═══════════════════════════════════════════════════════════════════

/// Per-field codegen context. Constructed once per struct/message field.
/// Caches common metadata eagerly; generates expressions lazily.
///
/// **Does not handle message-specific concerns** (Option wrapping, Box for
/// self-referential fields). Those are container-level layout decisions that
/// stay in `gen_message.rs`.
pub struct FieldCodegen<'a> {
  td: &'a TypeDescriptor<'a>,
  analysis: &'a SchemaAnalysis,
  /// Cow-aware type for field declarations (e.g. `Cow<'buf, str>`).
  cow_type: String,
  /// Owned type for constructor parameters (e.g. `string::String`).
  owned_type: String,
  /// Fixed encoded size expression, if the type is fixed-size.
  wire_size_expr: Option<String>,
  /// Whether this field is directly Cow-wrapped
  /// (String, byte array, bulk scalar array).
  is_cow: bool,
}

impl<'a> FieldCodegen<'a> {
  /// Construct a new `FieldCodegen` for a field's type descriptor.
  pub fn new(
    td: &'a TypeDescriptor<'a>,
    analysis: &'a SchemaAnalysis,
  ) -> Result<Self, GeneratorError> {
    let cow_type = compute_cow_type(td, analysis)?;
    let owned_type = compute_owned_type(td, analysis)?;
    let wire_size_expr = compute_wire_size_expr(td)?;
    let is_cow = is_cow_field(td);
    Ok(Self {
      td,
      analysis,
      cow_type,
      owned_type,
      wire_size_expr,
      is_cow,
    })
  }

  // ── Cached metadata ─────────────────────────────────────────────

  /// Cow-aware field type string (e.g. `borrow::Cow<'buf, str>`, `i32`).
  pub fn cow_type(&self) -> &str {
    &self.cow_type
  }

  /// Owned type string for `new()` constructors
  /// (e.g. `string::String`, `i32`, `TypeName<'static>`).
  #[allow(dead_code)]
  pub fn owned_type(&self) -> &str {
    &self.owned_type
  }

  /// Fixed encoded size expression, if the type has a compile-time-known size.
  pub fn wire_size_expr(&self) -> Option<&str> {
    self.wire_size_expr.as_deref()
  }

  /// Whether this field is directly Cow-wrapped (String, byte[], bulk scalar[]).
  #[allow(dead_code)]
  pub fn is_cow(&self) -> bool {
    self.is_cow
  }

  /// Access the underlying type descriptor.
  #[allow(dead_code)]
  pub fn td(&self) -> &TypeDescriptor<'a> {
    self.td
  }

  // ── Expression generators (lazy) ────────────────────────────────

  /// Generate a write expression for this field.
  pub fn write_expr(&self, value: &str, writer: &str) -> Result<String, GeneratorError> {
    write_expression(self.td, value, writer, self.analysis)
  }

  /// Generate a decode expression with `.for_field()` error context.
  pub fn read_expr(
    &self,
    reader: &str,
    type_name: &str,
    field_name: &str,
  ) -> Result<String, GeneratorError> {
    read_field_expression(self.td, reader, self.analysis, type_name, field_name)
  }

  /// Generate an encoded size expression for this field.
  pub fn size_expr(&self, value: &str) -> Result<String, GeneratorError> {
    encoded_size_expression(self.td, value, self.analysis)
  }

  /// Generate a borrowed→owned conversion expression.
  pub fn owned_expr(&self, value: &str) -> Result<String, GeneratorError> {
    into_owned_expression(self.td, value, self.analysis)
  }

  /// Generate an owned→borrowed conversion expression for constructors.
  #[allow(dead_code)]
  pub fn borrowed_expr(&self, value: &str) -> Result<String, GeneratorError> {
    into_borrowed_expression(self.td, value, self.analysis)
  }

  /// Return constructor parameter info for this field.
  ///
  /// Handles IntoIterator for collections, `impl Into<Cow>` for strings/bytes,
  /// passthrough for scalars, and `into_borrowed` for defined types with lifetimes.
  pub fn constructor_param(
    &self,
    param_name: &str,
    has_lifetime: bool,
  ) -> Result<ConstructorParam, GeneratorError> {
    // 1. Collection fields — use IntoIterator
    if let Some((param_type, init_expr)) = collection_into_iter(self.td, param_name, self.analysis)?
    {
      return Ok(ConstructorParam {
        param_type,
        init_expr,
      });
    }

    // 2. Direct Cow fields (string, bytes, bulk scalar arrays) — use impl Into
    if self.is_cow {
      if has_lifetime {
        return Ok(ConstructorParam {
          param_type: format!("impl convert::Into<{}>", self.cow_type),
          init_expr: format!("{}.into()", param_name),
        });
      } else {
        // Non-lifetime case: wrap as Cow::Owned
        return Ok(ConstructorParam {
          param_type: self.owned_type.clone(),
          init_expr: format!("borrow::Cow::Owned({})", param_name),
        });
      }
    }

    // 3. Defined types with lifetime — may need into_borrowed
    if has_lifetime && self.analysis.type_needs_lifetime(self.td) {
      let expr = into_borrowed_expression(self.td, param_name, self.analysis)?;
      return Ok(ConstructorParam {
        param_type: self.owned_type.clone(),
        init_expr: expr,
      });
    }

    // 4. Everything else — pass through
    Ok(ConstructorParam {
      param_type: self.owned_type.clone(),
      init_expr: param_name.to_string(),
    })
  }
}

/// Constructor parameter type and initialization expression.
pub struct ConstructorParam {
  /// Parameter type string (e.g. `impl Into<Cow<'buf, str>>`, `i32`).
  pub param_type: String,
  /// Body expression to produce the field value from the parameter.
  pub init_expr: String,
}

// ═══════════════════════════════════════════════════════════════════
// Private helpers — type computation
// ═══════════════════════════════════════════════════════════════════

/// Returns true if a TypeDescriptor is directly a Cow-wrapped field
/// (String → Cow<str>, byte array → Cow<[u8]>, bulk scalar array → Cow<[T]>).
pub(super) fn is_cow_field(td: &TypeDescriptor) -> bool {
  let kind = match td.kind {
    Some(k) => k,
    None => return false,
  };
  match kind {
    TypeKind::String => true,
    TypeKind::Array => {
      if is_byte_array(td) {
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
fn is_byte_array(td: &TypeDescriptor) -> bool {
  td.kind == Some(TypeKind::Array)
    && td
      .array_element
      .as_ref()
      .is_some_and(|e| e.kind == Some(TypeKind::Byte))
}

/// Returns true if the TypeKind is safe for bulk memcpy.
pub(crate) fn is_bulk_scalar(kind: TypeKind) -> bool {
  fixed_scalar_info(kind).is_some_and(|s| s.is_bulk_eligible)
}

/// Map a scalar TypeKind to its Rust type string (including String).
fn scalar_type(kind: TypeKind) -> Option<&'static str> {
  if kind == TypeKind::String {
    return Some("string::String");
  }
  fixed_scalar_info(kind).map(|s| s.rust_type)
}

/// Map a scalar TypeKind to its Cow-aware type.
fn scalar_type_cow(kind: TypeKind) -> Option<&'static str> {
  if kind == TypeKind::String {
    return Some("borrow::Cow<'buf, str>");
  }
  fixed_scalar_info(kind).map(|s| s.rust_type)
}

/// Compute the Cow-aware Rust type string for a TypeDescriptor.
fn compute_cow_type(
  td: &TypeDescriptor,
  analysis: &SchemaAnalysis,
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
      if elem.kind == Some(TypeKind::Byte) {
        return Ok("bebop::BebopBytes<'buf>".to_string());
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        return Ok(format!("borrow::Cow<'buf, [{}]>", ty));
      }
      let inner = compute_cow_type(elem, analysis)?;
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
      let inner = compute_cow_type(elem, analysis)?;
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
      let k = compute_cow_type(key, analysis)?;
      let v = compute_cow_type(val, analysis)?;
      Ok(format!("bebop::HashMap<{}, {}>", k, v))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      if analysis.needs_lifetime(fqn) {
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

/// Compute the owned Rust type string for a TypeDescriptor.
fn compute_owned_type(
  td: &TypeDescriptor,
  analysis: &SchemaAnalysis,
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
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        return Ok(format!("vec::Vec<{}>", ty));
      }
      let inner = compute_owned_type(elem, analysis)?;
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
      let inner = compute_owned_type(elem, analysis)?;
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
      let k = compute_owned_type(key, analysis)?;
      let v = compute_owned_type(val, analysis)?;
      Ok(format!("bebop::HashMap<{}, {}>", k, v))
    }
    TypeKind::Defined => {
      let fqn = td
        .defined_fqn
        .as_deref()
        .ok_or_else(|| GeneratorError::MalformedType("defined type missing fqn".into()))?;
      let type_name = fqn_to_type_name(fqn);
      if analysis.needs_lifetime(fqn) {
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

/// Compute a constant Rust expression for a type's fixed encoded size.
/// Returns `Ok(None)` for variable-size types.
fn compute_wire_size_expr(td: &TypeDescriptor) -> Result<Option<String>, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if let Some(info) = fixed_scalar_info(kind) {
    return Ok(Some(info.wire_size_expr.to_string()));
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
      let elem_expr = compute_wire_size_expr(elem)?;
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
// Private helpers — expression generation
// ═══════════════════════════════════════════════════════════════════

/// Generate a raw zero-copy read expression (without `.for_field()` context).
#[allow(clippy::only_used_in_recursion)]
fn read_expression(
  td: &TypeDescriptor,
  reader: &str,
  analysis: &SchemaAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Scalars
  match kind {
    TypeKind::String => {
      return Ok(format!(
        "result::Result::Ok(borrow::Cow::Borrowed({}.read_str()?))",
        reader
      ));
    }
    _ => {
      if let Some(info) = fixed_scalar_info(kind) {
        return Ok(format!("{}.{}()", reader, info.read_method));
      }
    }
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!(
          "result::Result::Ok(bebop::BebopBytes::borrowed({}.read_byte_slice()?))",
          reader
        ));
      }
      let elem_kind = elem.kind.unwrap_or(TypeKind::Unknown);
      if is_bulk_scalar(elem_kind) {
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        return Ok(format!("{}.read_scalar_array::<{}>()", reader, ty));
      }
      if elem_kind == TypeKind::Defined {
        let fqn = elem.defined_fqn.as_deref().unwrap_or("");
        let type_name = fqn_to_type_name(fqn);
        Ok(format!(
          "{}.read_array(|_r| {}::decode(_r))",
          reader, type_name
        ))
      } else {
        let inner = read_expression(elem, "_r", analysis)?;
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
      if fixed_scalar_info(elem_kind).is_some_and(|s| s.is_fixed_array_scalar) {
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        Ok(format!("{}.read_fixed_array::<{}, {}>()", reader, ty, size))
      } else {
        let inner = read_expression(elem, reader, analysis)?;
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
      let k_expr = read_expression(key, "_r", analysis)?;
      let v_expr = read_expression(val, "_r", analysis)?;
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
      Ok(format!("{}::decode({})", type_name, reader))
    }
    _ => Err(GeneratorError::MalformedType(format!(
      "cannot generate cow read for type kind: {}",
      kind as u8
    ))),
  }
}

/// Generate a decode expression with `.for_field()` context applied.
fn read_field_expression(
  td: &TypeDescriptor,
  reader: &str,
  analysis: &SchemaAnalysis,
  type_name: &str,
  field_name: &str,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  if kind == TypeKind::String {
    Ok(format!(
      "borrow::Cow::Borrowed({}.read_str().for_field(\"{}\", \"{}\")?)",
      reader, type_name, field_name
    ))
  } else if is_byte_array(td) {
    Ok(format!(
      "bebop::BebopBytes::borrowed({}.read_byte_slice().for_field(\"{}\", \"{}\")?)",
      reader, type_name, field_name
    ))
  } else {
    let read_expr = read_expression(td, reader, analysis)?;
    Ok(format!(
      "{}.for_field(\"{}\", \"{}\")?",
      read_expr, type_name, field_name
    ))
  }
}

/// Generate a write expression for a TypeDescriptor.
#[allow(clippy::only_used_in_recursion)]
fn write_expression(
  td: &TypeDescriptor,
  value: &str,
  writer: &str,
  analysis: &SchemaAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // String — Cow<str> derefs to &str
  if kind == TypeKind::String {
    return Ok(format!("{}.write_string(&{})", writer, value));
  }

  // Fixed scalars
  if let Some(info) = fixed_scalar_info(kind) {
    return Ok(format!("{}.{}({})", writer, info.write_method, value));
  }

  match kind {
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
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
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        Ok(format!(
          "{}.write_scalar_array::<{}>(&{})",
          writer, ty, value
        ))
      } else if let Some(info) = fixed_scalar_info(elem_kind) {
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| _w.{}(*_el))",
          writer, value, info.write_method
        ))
      } else if elem_kind == TypeKind::String {
        Ok(format!(
          "{}.write_array(&{}, |_w, _el| _w.write_string(_el))",
          writer, value
        ))
      } else {
        let inner = write_expression(elem, "_el", "_w", analysis)?;
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
      if fixed_scalar_info(elem_kind).is_some_and(|s| s.is_fixed_array_scalar) {
        let size = td
          .fixed_array_size
          .ok_or_else(|| GeneratorError::MalformedType("fixed array missing size".into()))?;
        let ty = fixed_scalar_info(elem_kind).unwrap().rust_type;
        Ok(format!(
          "{}.write_fixed_array::<{}, {}>(&{})",
          writer, ty, size, value
        ))
      } else {
        let inner = write_expression(elem, "_el", writer, analysis)?;
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
      let k_kind = key.kind.unwrap_or(TypeKind::Unknown);
      let v_kind = val.kind.unwrap_or(TypeKind::Unknown);
      let k_val = if fixed_scalar_info(k_kind).is_some() {
        "*_k"
      } else {
        "_k"
      };
      let v_val = if fixed_scalar_info(v_kind).is_some() {
        "*_v"
      } else {
        "_v"
      };
      let k_write = write_expression(key, k_val, "_w", analysis)?;
      let v_write = write_expression(val, v_val, "_w", analysis)?;
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

fn collection_ref(value: &str) -> String {
  if value.starts_with("self.") {
    format!("&{}", value)
  } else {
    value.to_string()
  }
}

/// Generate an expression computing the encoded byte size of a value.
#[allow(clippy::only_used_in_recursion)]
fn encoded_size_expression(
  td: &TypeDescriptor,
  value: &str,
  analysis: &SchemaAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  // Fixed-size scalars
  if let Some(info) = fixed_scalar_info(kind) {
    return Ok(info.wire_size_expr.to_string());
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
      if let Some(info) = fixed_scalar_info(elem_kind) {
        Ok(format!(
          "bebop::wire_size::array_size({}, |_el| ({}))",
          items_ref, info.wire_size_expr
        ))
      } else {
        let inner = encoded_size_expression(elem, "_el", analysis)?;
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
      if let Some(info) = fixed_scalar_info(elem_kind) {
        Ok(format!(
          "{}usize * ({})",
          size as usize, info.wire_size_expr
        ))
      } else {
        let inner = encoded_size_expression(elem, "_el", analysis)?;
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
      let k_size = encoded_size_expression(key, "_k", analysis)?;
      let v_size = encoded_size_expression(val, "_v", analysis)?;
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

/// Generate a borrowed→owned conversion expression.
fn into_owned_expression(
  td: &TypeDescriptor,
  value: &str,
  analysis: &SchemaAnalysis,
) -> Result<String, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  match kind {
    TypeKind::String => Ok(format!("borrow::Cow::Owned({}.into_owned())", value)),
    TypeKind::Array => {
      let elem = td
        .array_element
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedType("array missing element type".into()))?;
      if elem.kind == Some(TypeKind::Byte) {
        return Ok(format!("{}.into_owned()", value));
      }
      if elem.kind.is_some_and(is_bulk_scalar) {
        return Ok(format!("borrow::Cow::Owned({}.into_owned())", value));
      }
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
        if analysis.needs_lifetime(fqn.as_ref()) {
          return Ok(format!("{}.into_owned()", value));
        }
      }
      Ok(value.to_string())
    }
    _ => Ok(value.to_string()),
  }
}

/// Generate an owned→borrowed conversion expression for constructors.
fn into_borrowed_expression(
  td: &TypeDescriptor,
  value: &str,
  analysis: &SchemaAnalysis,
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
    TypeKind::Defined => Ok(value.to_string()),
    _ => Ok(value.to_string()),
  }
}

// ═══════════════════════════════════════════════════════════════════
// IntoIterator constructor helpers
// ═══════════════════════════════════════════════════════════════════

/// For a single element type, return (param_fragment, needs_into).
fn element_into_info(
  td: &TypeDescriptor,
  analysis: &SchemaAnalysis,
) -> Result<(String, bool), GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

  match kind {
    TypeKind::String => Ok(("borrow::Cow<'buf, str>".to_string(), true)),
    TypeKind::Array => {
      if is_byte_array(td) {
        return Ok(("bebop::BebopBytes<'buf>".to_string(), true));
      }
      if let Some(elem) = td.array_element.as_ref() {
        if let Some(ek) = elem.kind.filter(|k| is_bulk_scalar(*k)) {
          let ty = fixed_scalar_info(ek).unwrap().rust_type;
          return Ok((format!("borrow::Cow<'buf, [{}]>", ty), true));
        }
      }
      Ok((compute_cow_type(td, analysis)?, false))
    }
    TypeKind::Map => Ok((compute_cow_type(td, analysis)?, false)),
    TypeKind::Defined => Ok((compute_cow_type(td, analysis)?, false)),
    _ => {
      if let Some(info) = fixed_scalar_info(kind) {
        Ok((info.rust_type.to_string(), false))
      } else {
        Ok((compute_cow_type(td, analysis)?, false))
      }
    }
  }
}

/// Return `IntoIterator`-based parameter type and body expression for a collection
/// field in a struct `new()` constructor.
///
/// Returns `Ok(Some((param_type, body_expr)))` for Array and Map fields.
/// Returns `Ok(None)` for non-collection fields or fields already handled by
/// `is_cow_field`.
fn collection_into_iter(
  td: &TypeDescriptor,
  param_name: &str,
  analysis: &SchemaAnalysis,
) -> Result<Option<(String, String)>, GeneratorError> {
  let kind = td
    .kind
    .ok_or_else(|| GeneratorError::MalformedType("type descriptor missing kind".into()))?;

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
        let param = format!(
          "impl iter::IntoIterator<Item = impl convert::Into<{}>>",
          elem_type
        );
        let body = format!("{}.into_iter().map(|_e| _e.into()).collect()", param_name);
        Ok(Some((param, body)))
      } else {
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

#[cfg(test)]
#[path = "field_codegen_tests.rs"]
mod tests;
