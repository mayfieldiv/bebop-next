use crate::error::DecodeError;
use crate::wire::BebopReader;

// All Bebop enums use the newtype struct pattern to gracefully handle unknown
// values on the wire (matching Swift's RawRepresentable approach).

// ── TypeKind ────────────────────────────────────────────────────

/// Scalar and compound type kinds (uint8 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeKind(pub u8);

#[allow(dead_code)]
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

  pub fn is_scalar(self) -> bool {
    self.0 >= 1 && self.0 <= 19
  }

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── DefinitionKind ──────────────────────────────────────────────

/// Named definition kinds (uint8 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionKind(pub u8);

#[allow(dead_code)]
impl DefinitionKind {
  pub const UNKNOWN: Self = Self(0);
  pub const ENUM: Self = Self(1);
  pub const STRUCT: Self = Self(2);
  pub const MESSAGE: Self = Self(3);
  pub const UNION: Self = Self(4);
  pub const SERVICE: Self = Self(5);
  pub const CONST: Self = Self(6);
  pub const DECORATOR: Self = Self(7);

  pub fn name(self) -> &'static str {
    match self {
      Self::ENUM => "enum",
      Self::STRUCT => "struct",
      Self::MESSAGE => "message",
      Self::UNION => "union",
      Self::SERVICE => "service",
      Self::CONST => "const",
      Self::DECORATOR => "decorator",
      _ => "unknown",
    }
  }

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── MethodType ──────────────────────────────────────────────────

/// Service method streaming modes (uint8 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodType(pub u8);

#[allow(dead_code)]
impl MethodType {
  pub const UNKNOWN: Self = Self(0);
  pub const UNARY: Self = Self(1);
  pub const SERVER_STREAM: Self = Self(2);
  pub const CLIENT_STREAM: Self = Self(3);
  pub const DUPLEX_STREAM: Self = Self(4);

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── Visibility ──────────────────────────────────────────────────

/// Definition visibility (uint8 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility(pub u8);

#[allow(dead_code)]
impl Visibility {
  pub const DEFAULT: Self = Self(0);
  pub const EXPORT: Self = Self(1);
  pub const LOCAL: Self = Self(2);

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── LiteralKind ─────────────────────────────────────────────────

/// Literal value kinds (uint8 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteralKind(pub u8);

#[allow(dead_code)]
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

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── DecoratorTarget ─────────────────────────────────────────────

/// Decorator target bitmask (uint8 @flags on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoratorTarget(pub u8);

#[allow(dead_code)]
impl DecoratorTarget {
  pub const NONE: Self = Self(0);
  pub const ENUM: Self = Self(1 << 0);
  pub const STRUCT: Self = Self(1 << 1);
  pub const MESSAGE: Self = Self(1 << 2);
  pub const UNION: Self = Self(1 << 3);
  pub const FIELD: Self = Self(1 << 4);
  pub const SERVICE: Self = Self(1 << 5);
  pub const METHOD: Self = Self(1 << 6);
  pub const BRANCH: Self = Self(1 << 7);
  pub const ALL: Self = Self(0xFF);

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }
}

// ── Edition ─────────────────────────────────────────────────────

/// Schema edition markers (int32 on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition(pub i32);

#[allow(dead_code)]
impl Edition {
  pub const UNKNOWN: Self = Self(0);
  pub const EDITION_2026: Self = Self(1000);

  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_i32()?))
  }
}
