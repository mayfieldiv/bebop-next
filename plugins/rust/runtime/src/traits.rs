use crate::{BebopReader, BebopWriter, DecodeError};

/// Trait for types that can be encoded into the Bebop wire format.
pub trait BebopEncode {
  /// If the encoded size is always the same, return `Some(size)`.
  /// Variable-size types return `None` (the default).
  const FIXED_ENCODED_SIZE: Option<usize> = None;

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
