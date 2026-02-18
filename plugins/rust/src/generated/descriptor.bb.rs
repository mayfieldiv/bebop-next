/// Scalar and compound type kinds.
/// Scalars (1-18) encode as fixed-size little-endian bytes. `BOOL` is 1 byte
/// (0x00 = false, nonzero = true). `INT16` is 2 bytes. `UUID` is 16 bytes.
/// Compound types (19-23) use additional TypeDescriptor fields:
/// - `ARRAY`: element type in `array_element`
/// - `FIXED_ARRAY`: element type + size in `fixed_array_element`, `fixed_array_size`
/// - `MAP`: key and value types in `map_key`, `map_value`
/// - `DEFINED`: referenced type FQN in `defined_fqn`
/// Value 0 is a sentinel. A valid TypeDescriptor never has `kind == UNKNOWN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeKind(pub u8);

#[allow(non_upper_case_globals)]
impl TypeKind {
  pub const UNKNOWN: Self = Self(0);
  pub const BOOL: Self = Self(1);
  pub const BYTE: Self = Self(2);
  pub const INT8: Self = Self(3);
  pub const INT16: Self = Self(4);
  pub const UINT16: Self = Self(5);
  pub const INT32: Self = Self(6);
  pub const UINT32: Self = Self(7);
  pub const INT64: Self = Self(8);
  pub const UINT64: Self = Self(9);
  pub const INT128: Self = Self(10);
  pub const UINT128: Self = Self(11);
  pub const FLOAT16: Self = Self(12);
  pub const FLOAT32: Self = Self(13);
  pub const FLOAT64: Self = Self(14);
  pub const BFLOAT16: Self = Self(15);
  pub const STRING: Self = Self(16);
  pub const UUID: Self = Self(17);
  pub const TIMESTAMP: Self = Self(18);
  pub const DURATION: Self = Self(19);
  pub const ARRAY: Self = Self(20);
  pub const FIXED_ARRAY: Self = Self(21);
  pub const MAP: Self = Self(22);
  pub const DEFINED: Self = Self(23);
}

impl TypeKind {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

/// Named definition kinds.
/// Each DefinitionDescriptor has one kind, which determines which body field
/// is populated. A definition with `kind == STRUCT` has `struct_def` set;
/// other body fields are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionKind(pub u8);

#[allow(non_upper_case_globals)]
impl DefinitionKind {
  pub const UNKNOWN: Self = Self(0);
  pub const ENUM: Self = Self(1);
  pub const STRUCT: Self = Self(2);
  pub const MESSAGE: Self = Self(3);
  pub const UNION: Self = Self(4);
  pub const SERVICE: Self = Self(5);
  pub const CONST: Self = Self(6);
  pub const DECORATOR: Self = Self(7);
}

impl DefinitionKind {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

/// Service method streaming modes.
/// Corresponds to `stream` keyword placement in method declarations:
/// ```bebop
/// Ping(Req): Res;               // UNARY
/// Watch(Req): stream Res;       // SERVER_STREAM
/// Upload(stream Chunk): Res;    // CLIENT_STREAM
/// Chat(stream Msg): stream Msg; // DUPLEX_STREAM
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodType(pub u8);

#[allow(non_upper_case_globals)]
impl MethodType {
  pub const UNKNOWN: Self = Self(0);
  pub const UNARY: Self = Self(1);
  pub const SERVER_STREAM: Self = Self(2);
  pub const CLIENT_STREAM: Self = Self(3);
  pub const DUPLEX_STREAM: Self = Self(4);
}

impl MethodType {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

/// Definition visibility.
/// Controls whether a definition is accessible from importing schemas.
/// Top-level definitions default to `EXPORT`. Nested definitions default
/// to `LOCAL` (visible only within the parent scope).
/// The `export` keyword on a nested definition makes it reachable as
/// `Parent.Child` from other schemas. Local definitions still appear in
/// descriptors because exported types may reference them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility(pub u8);

#[allow(non_upper_case_globals)]
impl Visibility {
  pub const DEFAULT: Self = Self(0);
  pub const EXPORT: Self = Self(1);
  pub const LOCAL: Self = Self(2);
}

impl Visibility {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

/// Literal value kinds.
/// Discriminates which value field of LiteralValue is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteralKind(pub u8);

#[allow(non_upper_case_globals)]
impl LiteralKind {
  pub const UNKNOWN: Self = Self(0);
  pub const BOOL: Self = Self(1);
  pub const INT: Self = Self(2);
  pub const FLOAT: Self = Self(3);
  pub const STRING: Self = Self(4);
  pub const UUID: Self = Self(5);
  pub const BYTES: Self = Self(6);
  pub const TIMESTAMP: Self = Self(7);
  pub const DURATION: Self = Self(8);
}

impl LiteralKind {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoratorTarget(pub u8);

#[allow(non_upper_case_globals)]
impl DecoratorTarget {
  pub const NONE: Self = Self(0);
  pub const ENUM: Self = Self(1);
  pub const STRUCT: Self = Self(2);
  pub const MESSAGE: Self = Self(4);
  pub const UNION: Self = Self(8);
  pub const FIELD: Self = Self(16);
  pub const SERVICE: Self = Self(32);
  pub const METHOD: Self = Self(64);
  pub const BRANCH: Self = Self(128);
  pub const ALL: Self = Self(255);
}

impl DecoratorTarget {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

impl std::ops::BitOr for DecoratorTarget {
  type Output = Self;
  fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl std::ops::BitAnd for DecoratorTarget {
  type Output = Self;
  fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
}

impl std::ops::Not for DecoratorTarget {
  type Output = Self;
  fn not(self) -> Self { Self(!self.0) }
}

/// Schema edition markers.
/// Edition values are ordered for comparison. Higher values are later editions.
/// A compiler rejects source files declaring an edition it does not support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition(pub i32);

#[allow(non_upper_case_globals)]
impl Edition {
  pub const UNKNOWN: Self = Self(0);
  pub const EDITION_2026: Self = Self(1000);
  pub const MAX: Self = Self(2147483647);
}

impl Edition {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_i32()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_i32(self.0);
  }

  pub fn encoded_size(&self) -> usize { 4 }
}

/// Type reference.
/// Every field type, const type, method parameter, array element, map key,
/// and map value in a descriptor is a TypeDescriptor. The `kind` field
/// determines which other fields are present:
/// - Scalars (`BOOL`..`DURATION`): no additional fields
/// - `ARRAY`: `array_element` is the element type
/// - `FIXED_ARRAY`: `fixed_array_element` + `fixed_array_size`
/// - `MAP`: `map_key` + `map_value`
/// - `DEFINED`: `defined_fqn` names the referenced type
/// TypeDescriptor is recursive. An array of maps:
/// ```
/// kind=ARRAY
/// array_element.kind=MAP
/// array_element.map_key.kind=STRING
/// array_element.map_value.kind=DEFINED
/// array_element.map_value.defined_fqn="mypackage.Item"
/// ```
#[derive(Debug, Clone, Default)]
pub struct TypeDescriptor {
/// Discriminates which fields below are populated.
  pub kind: Option<TypeKind>,
/// Element type when `kind == ARRAY`.
  pub array_element: Option<Box<TypeDescriptor>>,
/// Element type when `kind == FIXED_ARRAY`.
  pub fixed_array_element: Option<Box<TypeDescriptor>>,
/// Element count when `kind == FIXED_ARRAY`. Range 1-65535.
  pub fixed_array_size: Option<u32>,
/// Key type when `kind == MAP`. Must be hashable (bool, integers, string, uuid).
  pub map_key: Option<Box<TypeDescriptor>>,
/// Value type when `kind == MAP`.
  pub map_value: Option<Box<TypeDescriptor>>,
/// FQN when `kind == DEFINED`. Always fully qualified after linking.
  pub defined_fqn: Option<String>,
}

impl TypeDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.kind = Some(TypeKind::decode(reader)?),
        2 => msg.array_element = Some(Box::new(TypeDescriptor::decode(reader)?)),
        3 => msg.fixed_array_element = Some(Box::new(TypeDescriptor::decode(reader)?)),
        4 => msg.fixed_array_size = Some(reader.read_u32()?),
        5 => msg.map_key = Some(Box::new(TypeDescriptor::decode(reader)?)),
        6 => msg.map_value = Some(Box::new(TypeDescriptor::decode(reader)?)),
        7 => msg.defined_fqn = Some(reader.read_string()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.kind {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(ref v) = self.array_element {
      writer.write_tag(2);
      v.encode(writer);
    }
    if let Some(ref v) = self.fixed_array_element {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(v) = self.fixed_array_size {
      writer.write_tag(4);
      writer.write_u32(v);
    }
    if let Some(ref v) = self.map_key {
      writer.write_tag(5);
      v.encode(writer);
    }
    if let Some(ref v) = self.map_value {
      writer.write_tag(6);
      v.encode(writer);
    }
    if let Some(ref v) = self.defined_fqn {
      writer.write_tag(7);
      writer.write_string(&v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Concrete value.
/// Used in const definitions, decorator arguments, and decorator parameter
/// defaults/constraints. One typed value field is set, determined by `kind`.
/// The `raw_value` field preserves pre-expansion text for constants using
/// environment variable substitution (`$(...)`).
#[derive(Debug, Clone, Default)]
pub struct LiteralValue {
  pub kind: Option<LiteralKind>,
  pub bool_value: Option<bool>,
  pub int_value: Option<i64>,
  pub float_value: Option<f64>,
  pub string_value: Option<String>,
  pub uuid_value: Option<[u8; 16]>,
/// Original source text before `$(...)` expansion. Only set for string
/// literals that contained environment variable references.
  pub raw_value: Option<String>,
/// When `kind == BYTES`.
  pub bytes_value: Option<Vec<u8>>,
/// When `kind == TIMESTAMP`.
  pub timestamp_value: Option<(i64, i32)>,
/// When `kind == DURATION`.
  pub duration_value: Option<(i64, i32)>,
}

impl LiteralValue {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.kind = Some(LiteralKind::decode(reader)?),
        2 => msg.bool_value = Some(reader.read_bool()?),
        3 => msg.int_value = Some(reader.read_i64()?),
        4 => msg.float_value = Some(reader.read_f64()?),
        5 => msg.string_value = Some(reader.read_string()?),
        6 => msg.uuid_value = Some(reader.read_uuid()?),
        7 => msg.raw_value = Some(reader.read_string()?),
        8 => msg.bytes_value = Some(reader.read_byte_array()?),
        9 => msg.timestamp_value = Some(reader.read_timestamp()?),
        10 => msg.duration_value = Some(reader.read_duration()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.kind {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(v) = self.bool_value {
      writer.write_tag(2);
      writer.write_bool(v);
    }
    if let Some(v) = self.int_value {
      writer.write_tag(3);
      writer.write_i64(v);
    }
    if let Some(v) = self.float_value {
      writer.write_tag(4);
      writer.write_f64(v);
    }
    if let Some(ref v) = self.string_value {
      writer.write_tag(5);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.uuid_value {
      writer.write_tag(6);
      writer.write_uuid(&v);
    }
    if let Some(ref v) = self.raw_value {
      writer.write_tag(7);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.bytes_value {
      writer.write_tag(8);
      writer.write_byte_array(&v);
    }
    if let Some(v) = self.timestamp_value {
      writer.write_tag(9);
      writer.write_timestamp(v);
    }
    if let Some(v) = self.duration_value {
      writer.write_tag(10);
      writer.write_duration(v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Named or positional argument in a decorator usage.
/// Positional arguments have an empty string for `name`.
#[derive(Debug, Clone)]
pub struct DecoratorArg {
  pub name: String,
  pub value: LiteralValue,
}

impl DecoratorArg {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let name = reader.read_string()?;
    let value = LiteralValue::decode(reader)?;
    Ok(DecoratorArg {
      name,
      value,
    })
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_string(&self.name);
    self.value.encode(writer);
  }
}

/// Decorator applied to a definition, field, enum member, branch, or method.
/// The `fqn` identifies which decorator definition this usage corresponds to.
/// Always fully qualified after linking.
/// The `export_data` map holds key-value pairs from executing the decorator's
/// Lua export block. Generators read these to access decorator-computed
/// metadata without re-running Lua.
#[derive(Debug, Clone, Default)]
pub struct DecoratorUsage {
/// FQN of the decorator definition (e.g., `validators.range`).
  pub fqn: Option<String>,
/// Arguments passed at the usage site, in declaration order.
  pub args: Option<Vec<DecoratorArg>>,
/// Results from the decorator's export block.
  pub export_data: Option<std::collections::HashMap<String, LiteralValue>>,
}

impl DecoratorUsage {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.fqn = Some(reader.read_string()?),
        2 => msg.args = Some(reader.read_array(DecoratorArg::decode)?),
        3 => msg.export_data = Some(reader.read_map(|_r| Ok((_r.read_string()?, LiteralValue::decode(_r)?)))?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.fqn {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.args {
      writer.write_tag(2);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.export_data {
      writer.write_tag(3);
      writer.write_map(&v, |_w, _k, _v| { _w.write_string(&_k); _v.encode(_w); });
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Field in a struct or message.
/// Struct and message fields share this type, distinguished by `index`:
/// - Struct fields: `index == 0` (positional encoding, no tag byte)
/// - Message fields: `index` 1-255 (wire tag from source)
/// Struct fields encode in declaration order with no separators. Message
/// fields encode as (tag, value) pairs in any order, terminated by 0.
#[derive(Debug, Clone, Default)]
pub struct FieldDescriptor {
  pub name: Option<String>,
/// Text from preceding `///` comments in source.
  pub documentation: Option<String>,
  pub r#type: Option<TypeDescriptor>,
/// Wire tag: 0 for struct fields, 1-255 for message fields.
  pub index: Option<u32>,
  pub decorators: Option<Vec<DecoratorUsage>>,
}

impl FieldDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.name = Some(reader.read_string()?),
        2 => msg.documentation = Some(reader.read_string()?),
        3 => msg.r#type = Some(TypeDescriptor::decode(reader)?),
        4 => msg.index = Some(reader.read_u32()?),
        5 => msg.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.name {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.documentation {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.r#type {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(v) = self.index {
      writer.write_tag(4);
      writer.write_u32(v);
    }
    if let Some(ref v) = self.decorators {
      writer.write_tag(5);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Enum member (name + integer value).
/// The `value` is stored as uint64 regardless of the enum's base type.
/// For signed base types, sign-extend from the base type's bit width.
/// A member with value 0xFFFFFFFFFFFFFFFF in an int8-based enum is -1.
#[derive(Debug, Clone, Default)]
pub struct EnumMemberDescriptor {
  pub name: Option<String>,
  pub documentation: Option<String>,
/// Stored unsigned. Reinterpret per the parent enum's `base_type`.
  pub value: Option<u64>,
  pub decorators: Option<Vec<DecoratorUsage>>,
/// Original expression text (e.g., `1 << 3`). Absent for simple literals.
  pub value_expr: Option<String>,
}

impl EnumMemberDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.name = Some(reader.read_string()?),
        2 => msg.documentation = Some(reader.read_string()?),
        3 => msg.value = Some(reader.read_u64()?),
        4 => msg.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        5 => msg.value_expr = Some(reader.read_string()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.name {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.documentation {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(v) = self.value {
      writer.write_tag(3);
      writer.write_u64(v);
    }
    if let Some(ref v) = self.decorators {
      writer.write_tag(4);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.value_expr {
      writer.write_tag(5);
      writer.write_string(&v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Union branch.
/// Each branch has a discriminator byte (1-255) written before the payload.
/// Two modes:
/// **Inline branch**: body declared in the union.
/// ```bebop
/// union Shape { Circle(1): { radius: float32; } }
/// ```
/// Sets `inline_fqn = "mypackage.Shape.Circle"`. The inline definition
/// lives in the parent's `nested` array.
/// **Type-reference branch**: references an existing type.
/// ```bebop
/// union Shape { rect(2): Rect }
/// ```
/// Sets `type_ref_fqn = "mypackage.Rect"` and `name = "rect"`.
#[derive(Debug, Clone, Default)]
pub struct UnionBranchDescriptor {
/// Wire discriminator byte. Range 1-255.
  pub discriminator: Option<u8>,
  pub documentation: Option<String>,
/// FQN of inline definition. Mutually exclusive with `type_ref_fqn`.
  pub inline_fqn: Option<String>,
/// FQN of referenced type. Mutually exclusive with `inline_fqn`.
  pub type_ref_fqn: Option<String>,
/// Branch name for type-reference branches.
  pub name: Option<String>,
  pub decorators: Option<Vec<DecoratorUsage>>,
}

impl UnionBranchDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.discriminator = Some(reader.read_byte()?),
        2 => msg.documentation = Some(reader.read_string()?),
        3 => msg.inline_fqn = Some(reader.read_string()?),
        4 => msg.type_ref_fqn = Some(reader.read_string()?),
        5 => msg.name = Some(reader.read_string()?),
        6 => msg.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(v) = self.discriminator {
      writer.write_tag(1);
      writer.write_byte(v);
    }
    if let Some(ref v) = self.documentation {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.inline_fqn {
      writer.write_tag(3);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.type_ref_fqn {
      writer.write_tag(4);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.name {
      writer.write_tag(5);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.decorators {
      writer.write_tag(6);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Service method.
/// The `id` is MurmurHash3 of `/ServiceName/MethodName`, computed at compile
/// time. Gives a stable 32-bit routing key without transmitting the full name.
#[derive(Debug, Clone, Default)]
pub struct MethodDescriptor {
  pub name: Option<String>,
  pub documentation: Option<String>,
  pub request_type: Option<TypeDescriptor>,
  pub response_type: Option<TypeDescriptor>,
  pub method_type: Option<MethodType>,
/// MurmurHash3 of `/ServiceName/MethodName`.
  pub id: Option<u32>,
  pub decorators: Option<Vec<DecoratorUsage>>,
}

impl MethodDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.name = Some(reader.read_string()?),
        2 => msg.documentation = Some(reader.read_string()?),
        3 => msg.request_type = Some(TypeDescriptor::decode(reader)?),
        4 => msg.response_type = Some(TypeDescriptor::decode(reader)?),
        5 => msg.method_type = Some(MethodType::decode(reader)?),
        6 => msg.id = Some(reader.read_u32()?),
        7 => msg.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.name {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.documentation {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.request_type {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(ref v) = self.response_type {
      writer.write_tag(4);
      v.encode(writer);
    }
    if let Some(ref v) = self.method_type {
      writer.write_tag(5);
      v.encode(writer);
    }
    if let Some(v) = self.id {
      writer.write_tag(6);
      writer.write_u32(v);
    }
    if let Some(ref v) = self.decorators {
      writer.write_tag(7);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Enum definition body.
/// The `base_type` indicates the declared integer width (e.g., `: uint8`).
/// Defaults to `UINT32` when no suffix is specified. Valid base types:
/// `BYTE`, `INT8`, `INT16`, `UINT16`, `INT32`, `UINT32`, `INT64`, `UINT64`.
/// Member values are stored as uint64 and reinterpreted per `base_type`.
#[derive(Debug, Clone, Default)]
pub struct EnumDef {
  pub base_type: Option<TypeKind>,
  pub members: Option<Vec<EnumMemberDescriptor>>,
/// True when `@flags` is applied. Members are bit positions for OR.
  pub is_flags: Option<bool>,
}

impl EnumDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.base_type = Some(TypeKind::decode(reader)?),
        2 => msg.members = Some(reader.read_array(EnumMemberDescriptor::decode)?),
        3 => msg.is_flags = Some(reader.read_bool()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.base_type {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(ref v) = self.members {
      writer.write_tag(2);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(v) = self.is_flags {
      writer.write_tag(3);
      writer.write_bool(v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Struct definition body.
/// Fields encode positionally: bytes concatenated in declaration order,
/// no tags, no length prefix (for fixed-size structs). Variable-size structs
/// get a u32 byte-length prefix.
#[derive(Debug, Clone, Default)]
pub struct StructDef {
  pub fields: Option<Vec<FieldDescriptor>>,
/// True when declared with `mut`. Mutable structs allow field reassignment.
  pub is_mutable: Option<bool>,
/// Total wire bytes when all fields are fixed-size. Zero when any field
/// is variable-size. Generators use this to pre-allocate buffers.
  pub fixed_size: Option<u32>,
}

impl StructDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.fields = Some(reader.read_array(FieldDescriptor::decode)?),
        2 => msg.is_mutable = Some(reader.read_bool()?),
        3 => msg.fixed_size = Some(reader.read_u32()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.fields {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(v) = self.is_mutable {
      writer.write_tag(2);
      writer.write_bool(v);
    }
    if let Some(v) = self.fixed_size {
      writer.write_tag(3);
      writer.write_u32(v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Message definition body.
/// Fields encode as tagged pairs (index, value), terminated by zero byte.
/// Fields can be added or removed without breaking existing decoders.
#[derive(Debug, Clone, Default)]
pub struct MessageDef {
  pub fields: Option<Vec<FieldDescriptor>>,
}

impl MessageDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.fields = Some(reader.read_array(FieldDescriptor::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.fields {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Union definition body.
/// Wire encoding: u32 byte_length, discriminator byte, branch payload.
/// Decoders read the length to skip unknown discriminators.
#[derive(Debug, Clone, Default)]
pub struct UnionDef {
  pub branches: Option<Vec<UnionBranchDescriptor>>,
}

impl UnionDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.branches = Some(reader.read_array(UnionBranchDescriptor::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.branches {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Service definition body.
#[derive(Debug, Clone, Default)]
pub struct ServiceDef {
  pub methods: Option<Vec<MethodDescriptor>>,
}

impl ServiceDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.methods = Some(reader.read_array(MethodDescriptor::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.methods {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Const definition body.
/// Constants are evaluated at parse time. String constants support
/// environment variable substitution:
/// ```bebop
/// const API_KEY : string = $(API_KEY);
/// ```
/// The LiteralValue's `raw_value` preserves `$(API_KEY)` while `string_value`
/// holds the expanded result.
#[derive(Debug, Clone, Default)]
pub struct ConstDef {
  pub r#type: Option<TypeDescriptor>,
  pub value: Option<LiteralValue>,
}

impl ConstDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.r#type = Some(TypeDescriptor::decode(reader)?),
        2 => msg.value = Some(LiteralValue::decode(reader)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.r#type {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(ref v) = self.value {
      writer.write_tag(2);
      v.encode(writer);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Decorator parameter definition.
/// Defines the schema for one argument a decorator accepts:
/// ```bebop
/// #decorator(validate) {
///   param min!: float64       // required, no default
///   param max?: float64 = 100 // optional, default 100
///   param mode?: string in ["strict", "lenient"]
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DecoratorParamDef {
  pub name: Option<String>,
/// Description from the `///` comment preceding this param.
  pub description: Option<String>,
/// Must be a scalar TypeKind.
  pub r#type: Option<TypeKind>,
/// True for required (`!`) params, false for optional (`?`).
  pub required: Option<bool>,
/// Default value for optional params. Absent for required params.
  pub default_value: Option<LiteralValue>,
/// Allowed-value constraint from `in [...]`. When non-empty, arguments
/// must match one of these values.
  pub allowed_values: Option<Vec<LiteralValue>>,
}

impl DecoratorParamDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.name = Some(reader.read_string()?),
        2 => msg.description = Some(reader.read_string()?),
        3 => msg.r#type = Some(TypeKind::decode(reader)?),
        4 => msg.required = Some(reader.read_bool()?),
        5 => msg.default_value = Some(LiteralValue::decode(reader)?),
        6 => msg.allowed_values = Some(reader.read_array(LiteralValue::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.name {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.description {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.r#type {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(v) = self.required {
      writer.write_tag(4);
      writer.write_bool(v);
    }
    if let Some(ref v) = self.default_value {
      writer.write_tag(5);
      v.encode(writer);
    }
    if let Some(ref v) = self.allowed_values {
      writer.write_tag(6);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Decorator definition body.
/// Describes the decorator contract: which elements it can target, what
/// parameters it accepts, and optional Lua source for validate/export.
#[derive(Debug, Clone, Default)]
pub struct DecoratorDef {
/// Bitmask of DecoratorTarget values this decorator may apply to.
  pub targets: Option<DecoratorTarget>,
/// When true, the decorator can appear multiple times on the same target.
  pub allow_multiple: Option<bool>,
  pub params: Option<Vec<DecoratorParamDef>>,
/// Lua source for validate block. Runs at compile time to reject invalid
/// usages. Absent when the decorator has no validate block.
  pub validate_source: Option<String>,
/// Lua source for export block. Produces key-value metadata stored in
/// DecoratorUsage.export_data. Absent when no export block.
  pub export_source: Option<String>,
}

impl DecoratorDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.targets = Some(DecoratorTarget::decode(reader)?),
        2 => msg.allow_multiple = Some(reader.read_bool()?),
        3 => msg.params = Some(reader.read_array(DecoratorParamDef::decode)?),
        4 => msg.validate_source = Some(reader.read_string()?),
        5 => msg.export_source = Some(reader.read_string()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.targets {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(v) = self.allow_multiple {
      writer.write_tag(2);
      writer.write_bool(v);
    }
    if let Some(ref v) = self.params {
      writer.write_tag(3);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.validate_source {
      writer.write_tag(4);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.export_source {
      writer.write_tag(5);
      writer.write_string(&v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Named definition.
/// Container for all definition kinds. The `kind` field selects which body
/// field is populated. Exactly one body is present per definition.
/// The `name` field is the simple identifier from source (`ChatMessage`).
/// The `fqn` includes package and parent scopes (`myapp.Outer.ChatMessage`).
/// Nested definitions (types declared inside struct/message/union bodies)
/// live in the `nested` array. Inline union branch definitions also appear
/// there. Each nested definition has its own FQN encoding the full path.
#[derive(Debug, Clone, Default)]
pub struct DefinitionDescriptor {
  pub kind: Option<DefinitionKind>,
/// Simple name as declared in source.
  pub name: Option<String>,
/// Fully-qualified name including package and parent scopes.
  pub fqn: Option<String>,
/// Text from preceding `///` comments in source.
  pub documentation: Option<String>,
  pub visibility: Option<Visibility>,
  pub decorators: Option<Vec<DecoratorUsage>>,
/// Types declared inside this definition's body.
  pub nested: Option<Vec<DefinitionDescriptor>>,
  pub enum_def: Option<EnumDef>,
  pub struct_def: Option<StructDef>,
  pub message_def: Option<MessageDef>,
  pub union_def: Option<UnionDef>,
  pub service_def: Option<ServiceDef>,
  pub const_def: Option<ConstDef>,
  pub decorator_def: Option<DecoratorDef>,
}

impl DefinitionDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.kind = Some(DefinitionKind::decode(reader)?),
        2 => msg.name = Some(reader.read_string()?),
        3 => msg.fqn = Some(reader.read_string()?),
        4 => msg.documentation = Some(reader.read_string()?),
        5 => msg.visibility = Some(Visibility::decode(reader)?),
        6 => msg.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        7 => msg.nested = Some(reader.read_array(DefinitionDescriptor::decode)?),
        8 => msg.enum_def = Some(EnumDef::decode(reader)?),
        9 => msg.struct_def = Some(StructDef::decode(reader)?),
        10 => msg.message_def = Some(MessageDef::decode(reader)?),
        11 => msg.union_def = Some(UnionDef::decode(reader)?),
        12 => msg.service_def = Some(ServiceDef::decode(reader)?),
        13 => msg.const_def = Some(ConstDef::decode(reader)?),
        14 => msg.decorator_def = Some(DecoratorDef::decode(reader)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.kind {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(ref v) = self.name {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.fqn {
      writer.write_tag(3);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.documentation {
      writer.write_tag(4);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.visibility {
      writer.write_tag(5);
      v.encode(writer);
    }
    if let Some(ref v) = self.decorators {
      writer.write_tag(6);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.nested {
      writer.write_tag(7);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.enum_def {
      writer.write_tag(8);
      v.encode(writer);
    }
    if let Some(ref v) = self.struct_def {
      writer.write_tag(9);
      v.encode(writer);
    }
    if let Some(ref v) = self.message_def {
      writer.write_tag(10);
      v.encode(writer);
    }
    if let Some(ref v) = self.union_def {
      writer.write_tag(11);
      v.encode(writer);
    }
    if let Some(ref v) = self.service_def {
      writer.write_tag(12);
      v.encode(writer);
    }
    if let Some(ref v) = self.const_def {
      writer.write_tag(13);
      v.encode(writer);
    }
    if let Some(ref v) = self.decorator_def {
      writer.write_tag(14);
      v.encode(writer);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Source location for a descriptor element.
/// Maps descriptor elements to source positions and comments. Used for
/// error reporting, documentation extraction, and IDE features.
#[derive(Debug, Clone, Default)]
pub struct Location {
/// Path into the descriptor tree. Encoded as pairs of (field_tag, index):
/// ```
/// [5, 0]       -> schema.definitions[0]
/// [5, 0, 1, 2] -> schema.definitions[0].fields[2]
/// [5, 1, 2, 0] -> schema.definitions[1].members[0]
/// ```
/// Field tags correspond to definition body field indices
/// (StructDef.fields = 1, EnumDef.members = 2, etc.).
  pub path: Option<Vec<i32>>,
/// Source span as `[start_line, start_col, end_line, end_col]`.
/// All values 1-based. Columns count characters, tabs advance to
/// next multiple of 4.
  pub span: Option<[i32; 4]>,
/// Comments on adjacent preceding lines with no blank line separation.
  pub leading_comments: Option<String>,
/// Comment on the same line after the element or after an opening brace.
  pub trailing_comments: Option<String>,
/// Comment groups separated from the element by blank lines.
  pub detached_comments: Option<Vec<String>>,
}

impl Location {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.path = Some(reader.read_array(|_r| _r.read_i32())?),
        2 => msg.span = Some(reader.read_fixed_i32_array::<4>()?),
        3 => msg.leading_comments = Some(reader.read_string()?),
        4 => msg.trailing_comments = Some(reader.read_string()?),
        5 => msg.detached_comments = Some(reader.read_array(|_r| _r.read_string())?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.path {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _w.write_i32(*_el));
    }
    if let Some(ref v) = self.span {
      writer.write_tag(2);
      writer.write_fixed_i32_array::<4>(&v);
    }
    if let Some(ref v) = self.leading_comments {
      writer.write_tag(3);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.trailing_comments {
      writer.write_tag(4);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.detached_comments {
      writer.write_tag(5);
      writer.write_array(&v, |_w, _el| _w.write_string(_el));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Source code info for a schema.
/// One Location entry per locatable element. Includes definitions, fields,
/// enum members, union branches, and service methods. Not every element
/// has a location.
#[derive(Debug, Clone, Default)]
pub struct SourceCodeInfo {
  pub locations: Option<Vec<Location>>,
}

impl SourceCodeInfo {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.locations = Some(reader.read_array(Location::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.locations {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Descriptor for a single .bop source file.
/// Each source file produces one SchemaDescriptor. Definitions are
/// topologically sorted: every type appears after the types it depends on.
/// Generators can emit types in array order without forward declarations.
#[derive(Debug, Clone, Default)]
pub struct SchemaDescriptor {
/// File path as provided to the compiler.
  pub path: Option<String>,
/// Package declaration from source. Absent when no package is declared.
  pub package: Option<String>,
  pub edition: Option<Edition>,
/// Import paths in source declaration order.
  pub imports: Option<Vec<String>>,
/// All definitions in topological dependency order.
  pub definitions: Option<Vec<DefinitionDescriptor>>,
/// Source code info. Only present when requested during compilation.
  pub source_code_info: Option<SourceCodeInfo>,
}

impl SchemaDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.path = Some(reader.read_string()?),
        2 => msg.package = Some(reader.read_string()?),
        3 => msg.edition = Some(Edition::decode(reader)?),
        4 => msg.imports = Some(reader.read_array(|_r| _r.read_string())?),
        5 => msg.definitions = Some(reader.read_array(DefinitionDescriptor::decode)?),
        6 => msg.source_code_info = Some(SourceCodeInfo::decode(reader)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.path {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.package {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.edition {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(ref v) = self.imports {
      writer.write_tag(4);
      writer.write_array(&v, |_w, _el| _w.write_string(_el));
    }
    if let Some(ref v) = self.definitions {
      writer.write_tag(5);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.source_code_info {
      writer.write_tag(6);
      v.encode(writer);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Root container for compiled schemas.
/// Written to .bop.bin files or passed to code generators. Schemas are
/// ordered so imported schemas appear before schemas that import them.
/// Processing schemas[0..N] in order encounters dependencies before
/// they are referenced.
#[derive(Debug, Clone, Default)]
pub struct DescriptorSet {
  pub schemas: Option<Vec<SchemaDescriptor>>,
}

impl DescriptorSet {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.schemas = Some(reader.read_array(SchemaDescriptor::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.schemas {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

