#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use core::ops::{BitAnd, BitOr, BitXor, Not};

use crate::{BebopReader, DecodeError};
#[cfg(feature = "alloc")]
use crate::BebopWriter;

/// Trait for types that can be encoded into the Bebop wire format.
#[cfg(feature = "alloc")]
pub trait BebopEncode {
  /// Encode `self` into the writer.
  fn encode(&self, writer: &mut BebopWriter);

  /// Compute the exact encoded byte size of `self`.
  fn encoded_size(&self) -> usize;

  /// Convenience: encode into a fresh `Vec<u8>`.
  fn to_bytes(&self) -> Vec<u8> {
    let mut writer = BebopWriter::with_capacity(self.encoded_size());
    self.encode(&mut writer);
    writer.into_bytes()
  }
}

/// Trait for types that can be decoded from the Bebop wire format.
///
/// The lifetime `'buf` ties borrowed fields (e.g. `Cow<'buf, str>`) to the
/// input buffer, enabling zero-copy deserialization.
pub trait BebopDecode<'buf>: Sized {
  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError>;

  /// Convenience: decode from a byte slice.
  fn from_bytes(buf: &'buf [u8]) -> Result<Self, DecodeError> {
    let mut reader = BebopReader::new(buf);
    Self::decode(&mut reader)
  }
}

/// Marker trait for types that can be decoded from any lifetime.
///
/// Automatically implemented for any `T: for<'buf> BebopDecode<'buf>`.
/// This is useful for types with no borrowed fields (all-owned data).
pub trait BebopDecodeOwned: for<'buf> BebopDecode<'buf> {}
impl<T: for<'buf> BebopDecode<'buf>> BebopDecodeOwned for T {}

/// Primitive integer types usable as flag bit storage.
#[cfg(feature = "alloc")]
pub trait BebopFlagBits:
  Copy
  + Default
  + Eq
  + BitAnd<Output = Self>
  + BitOr<Output = Self>
  + BitXor<Output = Self>
  + Not<Output = Self>
{
  const FIXED_ENCODED_SIZE: usize;

  fn encode_bits(self, writer: &mut BebopWriter);
  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError>;
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for u8 {
  const FIXED_ENCODED_SIZE: usize = 1;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_byte(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_byte()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for i8 {
  const FIXED_ENCODED_SIZE: usize = 1;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_i8(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_i8()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for u16 {
  const FIXED_ENCODED_SIZE: usize = 2;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_u16(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_u16()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for i16 {
  const FIXED_ENCODED_SIZE: usize = 2;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_i16(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_i16()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for u32 {
  const FIXED_ENCODED_SIZE: usize = 4;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_u32(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_u32()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for i32 {
  const FIXED_ENCODED_SIZE: usize = 4;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_i32(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_i32()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for u64 {
  const FIXED_ENCODED_SIZE: usize = 8;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_u64(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_u64()
  }
}

#[cfg(feature = "alloc")]
impl BebopFlagBits for i64 {
  const FIXED_ENCODED_SIZE: usize = 8;

  fn encode_bits(self, writer: &mut BebopWriter) {
    writer.write_i64(self);
  }

  fn decode_bits<'buf>(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    reader.read_i64()
  }
}

/// Shared behavior for generated `@flags` newtypes.
#[cfg(feature = "alloc")]
pub trait BebopFlags: Sized + Copy {
  type Bits: BebopFlagBits;

  const ALL_BITS: Self::Bits;

  fn bits(self) -> Self::Bits;
  fn from_bits_retain(bits: Self::Bits) -> Self;

  fn empty() -> Self {
    Self::from_bits_retain(Self::Bits::default())
  }

  fn all() -> Self {
    Self::from_bits_retain(Self::ALL_BITS)
  }

  fn from_bits(bits: Self::Bits) -> Option<Self> {
    if (bits & !Self::ALL_BITS) == Self::Bits::default() {
      Some(Self::from_bits_retain(bits))
    } else {
      None
    }
  }

  fn from_bits_truncate(bits: Self::Bits) -> Self {
    Self::from_bits_retain(bits & Self::ALL_BITS)
  }

  fn is_empty(self) -> bool {
    self.bits() == Self::Bits::default()
  }

  fn is_all(self) -> bool {
    self.bits() == Self::ALL_BITS
  }

  fn contains(self, other: Self) -> bool {
    (self.bits() & other.bits()) == other.bits()
  }

  fn intersects(self, other: Self) -> bool {
    (self.bits() & other.bits()) != Self::Bits::default()
  }

  fn insert(&mut self, other: Self) {
    *self = Self::from_bits_retain(self.bits() | other.bits());
  }

  fn remove(&mut self, other: Self) {
    *self = Self::from_bits_retain(self.bits() & !other.bits());
  }

  fn toggle(&mut self, other: Self) {
    *self = Self::from_bits_retain(self.bits() ^ other.bits());
  }
}

#[cfg(feature = "alloc")]
impl<T> BebopEncode for T
where
  T: BebopFlags + Copy,
{
  fn encode(&self, writer: &mut BebopWriter) {
    self.bits().encode_bits(writer);
  }

  fn encoded_size(&self) -> usize {
    <T::Bits as BebopFlagBits>::FIXED_ENCODED_SIZE
  }
}

#[cfg(feature = "alloc")]
impl<'buf, T> BebopDecode<'buf> for T
where
  T: BebopFlags,
{
  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {
    Ok(Self::from_bits_retain(T::Bits::decode_bits(reader)?))
  }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
  use alloc::vec;

  use super::{BebopDecode, BebopEncode, BebopFlags};

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  struct Permissions(u8);

  impl Permissions {
    const READ: Self = Self(1);
    const WRITE: Self = Self(2);
    const EXECUTE: Self = Self(4);
  }

  impl BebopFlags for Permissions {
    type Bits = u8;

    const ALL_BITS: Self::Bits = 7;

    fn bits(self) -> Self::Bits {
      self.0
    }

    fn from_bits_retain(bits: Self::Bits) -> Self {
      Self(bits)
    }
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  struct SignedFlags(i32);

  impl BebopFlags for SignedFlags {
    type Bits = i32;

    const ALL_BITS: Self::Bits = 7;

    fn bits(self) -> Self::Bits {
      self.0
    }

    fn from_bits_retain(bits: Self::Bits) -> Self {
      Self(bits)
    }
  }

  #[test]
  fn flags_default_helpers() {
    let mut perms = Permissions::from_bits(5).unwrap();
    assert!(perms.contains(Permissions::READ));
    assert!(perms.intersects(Permissions::EXECUTE));
    assert!(!perms.contains(Permissions::WRITE));

    perms.insert(Permissions::WRITE);
    assert_eq!(perms.bits(), 7);
    assert!(perms.is_all());

    perms.remove(Permissions::READ);
    assert_eq!(perms.bits(), 6);
    perms.toggle(Permissions::EXECUTE);
    assert_eq!(perms.bits(), 2);
  }

  #[test]
  fn flags_from_bits_validation() {
    assert_eq!(Permissions::from_bits(3), Some(Permissions(3)));
    assert_eq!(Permissions::from_bits(128), None);
    assert_eq!(Permissions::from_bits_truncate(255).bits(), 7);
    assert!(Permissions::empty().is_empty());
    assert_eq!(Permissions::all().bits(), 7);
  }

  #[test]
  fn flags_blanket_wire_impls() {
    let perms = Permissions(5);
    let bytes = perms.to_bytes();
    assert_eq!(bytes, vec![5]);
    let decoded = Permissions::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, perms);
    assert_eq!(perms.encoded_size(), 1);
  }

  #[test]
  fn signed_flags_validation() {
    assert_eq!(SignedFlags::from_bits(7), Some(SignedFlags(7)));
    assert_eq!(SignedFlags::from_bits(-1), None);
    assert_eq!(SignedFlags::from_bits_truncate(-1), SignedFlags(7));
  }
}
