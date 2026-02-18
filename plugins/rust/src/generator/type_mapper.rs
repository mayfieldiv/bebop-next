use crate::descriptor::{TypeDescriptor, TypeKind};

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
    // TODO: Proper half-float / bfloat16 types
    TypeKind::FLOAT16 => Some("u16 /* TODO: f16 */"),
    TypeKind::BFLOAT16 => Some("u16 /* TODO: bf16 */"),
    // TODO: Proper UUID / timestamp / duration types
    TypeKind::UUID => Some("[u8; 16] /* TODO: uuid type */"),
    TypeKind::TIMESTAMP => Some("(i64, i32) /* TODO: timestamp type */"),
    TypeKind::DURATION => Some("(i64, i32) /* TODO: duration type */"),
    _ => None,
  }
}

/// Map a full TypeDescriptor to its Rust type string.
///
/// Returns a Rust type expression for any Bebop type, including compound types.
pub fn rust_type(_td: &TypeDescriptor) -> String {
  // TODO: Implement full type mapping
  //
  // - Scalars: delegate to scalar_type()
  // - ARRAY: Vec<rust_type(element)>
  // - FIXED_ARRAY: [rust_type(element); size]
  // - MAP: HashMap<rust_type(key), rust_type(value)>
  // - DEFINED: look up FQN -> Rust type name via NamingPolicy
  //
  // This is one of the most important functions for code generation.

  "() /* TODO */".to_string()
}

/// Generate a reader expression for a TypeDescriptor.
///
/// Returns a Rust expression like `reader.read_i32()?` or
/// `Foo::decode(&mut reader)?` for defined types.
pub fn read_expression(_td: &TypeDescriptor) -> String {
  // TODO: Implement
  //
  // - Scalars: reader.read_xxx()?
  // - ARRAY (bulk scalar): reader.read_length_prefixed_array::<T>()?
  // - ARRAY (other): reader.read_array(|r| ...)?
  // - FIXED_ARRAY: read N elements in order
  // - MAP: reader.read_map(|r| (read_key, read_value))?
  // - DEFINED: TypeName::decode(&mut reader)?

  "todo!(\"read expression\")".to_string()
}

/// Generate a writer expression for a TypeDescriptor.
///
/// Returns a Rust statement like `writer.write_i32(value)` or
/// `value.encode(&mut writer)` for defined types.
pub fn write_expression(_td: &TypeDescriptor, _value: &str) -> String {
  // TODO: Implement
  //
  // - Scalars: writer.write_xxx(value)
  // - ARRAY: writer.write_array(value, |w, elem| ...)
  // - MAP: writer.write_map(value, |w, k, v| ...)
  // - DEFINED: value.encode(&mut writer)

  "todo!(\"write expression\")".to_string()
}

/// Generate a size expression for a TypeDescriptor.
pub fn size_expression(_td: &TypeDescriptor, _value: &str) -> String {
  // TODO: Implement
  //
  // - Fixed-size scalars: literal number (e.g., "4" for i32)
  // - String: 4 + value.len() + 1
  // - ARRAY: 4 + sum of element sizes
  // - MAP: 4 + sum of entry sizes
  // - DEFINED: value.encoded_size()

  "0 /* TODO */".to_string()
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
