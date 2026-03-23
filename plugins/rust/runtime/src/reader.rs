use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::Hash;

use crate::temporal::{BebopDuration, BebopTimestamp};
use crate::traits::BulkScalar;
use crate::traits::FixedScalar;
use crate::HashMap;
use crate::{bf16, f16, DecodeError};

/// Validate UTF-8 using SIMD-accelerated validation when available.
/// Returns the validated `&str` directly, avoiding `from_utf8_unchecked`.
#[inline]
fn validate_utf8(bytes: &[u8]) -> core::result::Result<&str, DecodeError> {
  simdutf8::basic::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)
}

type Result<T> = core::result::Result<T, DecodeError>;

/// Cursor-based reader for Bebop wire format over a byte slice.
///
/// All multi-byte integers are little-endian.
/// Strings are: u32 length + UTF-8 bytes + NUL byte.
/// Messages are: u32 body_length + tagged fields + 0x00 end marker.
/// Structs are: fields concatenated in declaration order (no tags, no length prefix).
pub struct BebopReader<'a> {
  buf: &'a [u8],
  pos: usize,
}

impl<'a> BebopReader<'a> {
  #[inline]
  #[must_use]
  pub fn new(buf: &'a [u8]) -> Self {
    Self { buf, pos: 0 }
  }

  #[inline]
  #[must_use]
  pub fn position(&self) -> usize {
    self.pos
  }

  #[inline]
  #[must_use]
  pub fn remaining(&self) -> usize {
    // INVARIANT: pos <= buf.len() (maintained by all methods).
    self.buf.len() - self.pos
  }

  #[inline]
  fn ensure(&self, count: usize) -> Result<()> {
    // INVARIANT: pos <= buf.len() (maintained by all methods).
    // Plain subtraction is safe under the invariant and will panic in
    // debug+release if it is ever violated, rather than silently wrapping.
    if count <= self.buf.len() - self.pos {
      Ok(())
    } else {
      Err(DecodeError::UnexpectedEof {
        needed: count,
        available: self.remaining(),
      })
    }
  }

  /// Read exactly N bytes into a fixed-size array with a single bounds check.
  #[inline]
  fn read_n<const N: usize>(&mut self) -> Result<[u8; N]> {
    self.ensure(N)?;
    // ensure() guarantees pos + N <= buf.len(); try_into cannot fail
    // because the slice length is exactly N (known at compile time).
    let arr = self.buf[self.pos..self.pos + N].try_into().unwrap();
    self.pos += N;
    Ok(arr)
  }

  // ── Primitives ──────────────────────────────────────────────

  #[inline]
  pub fn read_bool(&mut self) -> Result<bool> {
    if self.pos < self.buf.len() {
      let v = self.buf[self.pos];
      self.pos += 1;
      Ok(v != 0)
    } else {
      Err(DecodeError::UnexpectedEof {
        needed: 1,
        available: 0,
      })
    }
  }

  #[inline]
  pub fn read_byte(&mut self) -> Result<u8> {
    if self.pos < self.buf.len() {
      let v = self.buf[self.pos];
      self.pos += 1;
      Ok(v)
    } else {
      Err(DecodeError::UnexpectedEof {
        needed: 1,
        available: 0,
      })
    }
  }

  #[inline]
  pub fn read_i8(&mut self) -> Result<i8> {
    Ok(self.read_byte()? as i8)
  }

  #[inline]
  pub fn read_u16(&mut self) -> Result<u16> {
    Ok(u16::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_i16(&mut self) -> Result<i16> {
    Ok(i16::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_u32(&mut self) -> Result<u32> {
    Ok(u32::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_i32(&mut self) -> Result<i32> {
    Ok(i32::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_u64(&mut self) -> Result<u64> {
    Ok(u64::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_i64(&mut self) -> Result<i64> {
    Ok(i64::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_i128(&mut self) -> Result<i128> {
    Ok(i128::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_u128(&mut self) -> Result<u128> {
    Ok(u128::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_f16(&mut self) -> Result<f16> {
    Ok(f16::from_bits(self.read_u16()?))
  }

  #[inline]
  pub fn read_bf16(&mut self) -> Result<bf16> {
    Ok(bf16::from_bits(self.read_u16()?))
  }

  #[inline]
  pub fn read_f32(&mut self) -> Result<f32> {
    Ok(f32::from_le_bytes(self.read_n()?))
  }

  #[inline]
  pub fn read_f64(&mut self) -> Result<f64> {
    Ok(f64::from_le_bytes(self.read_n()?))
  }

  // ── String ──────────────────────────────────────────────────

  /// Read a Bebop string: u32 byte_count + UTF-8 bytes + NUL terminator.
  /// The u32 is the number of UTF-8 bytes (NOT including the trailing NUL).
  #[inline]
  pub fn read_string(&mut self) -> Result<String> {
    let len = self.read_u32()? as usize;
    // On 32-bit targets usize == u32, so len + 1 can overflow when
    // len == u32::MAX.  Use checked arithmetic to surface a clean error.
    let total = len.checked_add(1).ok_or(DecodeError::UnexpectedEof {
      needed: usize::MAX,
      available: self.remaining(),
    })?;
    self.ensure(total)?; // string bytes + NUL
    let str_bytes = &self.buf[self.pos..self.pos + len];
    self.pos += total; // advance past string bytes + NUL
    let s = validate_utf8(str_bytes)?;
    Ok(String::from(s))
  }

  // ── UUID ────────────────────────────────────────────────────

  #[inline]
  pub fn read_uuid(&mut self) -> Result<uuid::Uuid> {
    Ok(uuid::Uuid::from_bytes(self.read_n()?))
  }

  // ── Timestamp / Duration ────────────────────────────────────

  /// Read a timestamp: i64 seconds + i32 nanos (12 bytes total).
  #[inline]
  pub fn read_timestamp(&mut self) -> Result<BebopTimestamp> {
    let seconds = self.read_i64()?;
    let nanos = self.read_i32()?;
    Ok(BebopTimestamp { seconds, nanos })
  }

  /// Read a duration: i64 seconds + i32 nanos (12 bytes total).
  #[inline]
  pub fn read_duration(&mut self) -> Result<BebopDuration> {
    let seconds = self.read_i64()?;
    let nanos = self.read_i32()?;
    Ok(BebopDuration { seconds, nanos })
  }

  // ── Message helpers ─────────────────────────────────────────

  /// Read message body length (u32).
  #[inline]
  pub fn read_message_length(&mut self) -> Result<u32> {
    self.read_u32()
  }

  /// Read a message field tag (u8). Tag 0 = end marker.
  #[inline]
  pub fn read_tag(&mut self) -> Result<u8> {
    self.read_byte()
  }

  /// Skip `count` bytes.
  #[inline]
  pub fn skip(&mut self, count: usize) -> Result<()> {
    self.ensure(count)?;
    self.pos += count;
    Ok(())
  }

  // ── Collections ─────────────────────────────────────────────

  /// Read a dynamic array: u32 count + elements.
  #[inline]
  pub fn read_array<T>(
    &mut self,
    mut read_elem: impl FnMut(&mut Self) -> Result<T>,
  ) -> Result<Vec<T>> {
    let count = self.read_u32()? as usize;
    if count > self.remaining() {
      return Err(DecodeError::UnexpectedEof {
        needed: count,
        available: self.remaining(),
      });
    }
    let mut items: Vec<T> = Vec::new();
    items
      .try_reserve(count)
      .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
    // Use ptr::write + set_len instead of push to avoid per-element
    // capacity check. try_reserve guarantees capacity >= count and
    // ptr remains valid for the entire loop (no reallocation occurs).
    let ptr = items.as_mut_ptr();
    for i in 0..count {
      match read_elem(self) {
        Ok(elem) => {
          // SAFETY: i < count <= capacity. We write each index once.
          unsafe { ptr.add(i).write(elem) };
        }
        Err(e) => {
          // Drop already-initialized elements before returning error
          // SAFETY: elements 0..i were initialized by previous iterations.
          unsafe {
            items.set_len(i);
          }
          return Err(e);
        }
      }
    }
    // SAFETY: All count elements have been initialized above.
    unsafe {
      items.set_len(count);
    }
    Ok(items)
  }

  /// Read a scalar array, borrowing directly from the buffer when possible.
  ///
  /// Wire format: u32 count + `count * size_of::<T>()` bytes in LE order.
  /// On little-endian with aligned data, returns `Cow::Borrowed` (zero-copy).
  /// On little-endian with unaligned data, returns `Cow::Owned` (bulk memcpy).
  /// On big-endian, falls back to per-element reads into `Cow::Owned`.
  #[inline]
  pub fn read_scalar_array<T: BulkScalar>(&mut self) -> Result<Cow<'a, [T]>> {
    let count = self.read_u32()? as usize;
    if count == 0 {
      return Ok(Cow::Borrowed(&[]));
    }
    let elem_size = core::mem::size_of::<T>();
    let byte_len = count
      .checked_mul(elem_size)
      .ok_or(DecodeError::UnexpectedEof {
        needed: usize::MAX,
        available: self.remaining(),
      })?;
    self.ensure(byte_len)?;

    if cfg!(target_endian = "little") {
      // SAFETY: ensure(byte_len) guarantees pos + byte_len <= buf.len(),
      // so pos is within the allocation and add() is sound.
      let ptr = unsafe { self.buf.as_ptr().add(self.pos) };
      if ptr.align_offset(core::mem::align_of::<T>()) == 0 {
        // SAFETY: T: BulkScalar guarantees all bit patterns are valid and
        // size_of::<T>() == wire element size. On LE, wire bytes == memory
        // layout. align_offset confirmed pointer alignment. ensure() confirmed
        // byte_len bytes are available. The resulting slice borrows from
        // self.buf with lifetime 'a.
        let slice = unsafe { core::slice::from_raw_parts(ptr as *const T, count) };
        self.pos += byte_len;
        Ok(Cow::Borrowed(slice))
      } else {
        // Unaligned: bulk memcpy into Vec
        let mut vec = Vec::<T>::new();
        vec
          .try_reserve(count)
          .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
        // SAFETY: Same as above minus alignment — we copy into a properly
        // aligned Vec instead. try_reserve guarantees capacity >= count.
        // copy_nonoverlapping is safe because src (self.buf) and dst (vec)
        // don't overlap. ptr was computed via add() above (same SAFETY).
        unsafe {
          core::ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr() as *mut u8, byte_len);
          vec.set_len(count);
        }
        self.pos += byte_len;
        Ok(Cow::Owned(vec))
      }
    } else {
      let mut items = Vec::new();
      items
        .try_reserve(count)
        .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
      for _ in 0..count {
        items.push(T::read_from(self)?);
      }
      Ok(Cow::Owned(items))
    }
  }

  /// Read a dynamic map: u32 count + (key, value) pairs.
  #[inline]
  pub fn read_map<K: Eq + Hash, V>(
    &mut self,
    mut read_entry: impl FnMut(&mut Self) -> Result<(K, V)>,
  ) -> Result<HashMap<K, V>> {
    let count = self.read_u32()? as usize;
    if count > self.remaining() / 2 {
      return Err(DecodeError::UnexpectedEof {
        needed: count.saturating_mul(2),
        available: self.remaining(),
      });
    }
    let mut map = HashMap::new();
    map
      .try_reserve(count)
      .map_err(|_| DecodeError::AllocationFailed { requested: count })?;
    for _ in 0..count {
      let (k, v) = read_entry(self)?;
      map.insert(k, v);
    }
    Ok(map)
  }

  /// Read a fixed-size array of any scalar type (no length prefix).
  #[inline]
  pub fn read_fixed_array<T: FixedScalar, const N: usize>(&mut self) -> Result<[T; N]> {
    let mut arr = [T::default(); N];
    for item in &mut arr {
      *item = T::read_from(self)?;
    }
    Ok(arr)
  }

  /// Read a fixed-size i32 array (no length prefix).
  #[inline]
  pub fn read_fixed_i32_array<const N: usize>(&mut self) -> Result<[i32; N]> {
    self.read_fixed_array::<i32, N>()
  }

  /// Read raw bytes.
  #[inline]
  pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
    self.ensure(count)?;
    let bytes = self.buf[self.pos..self.pos + count].to_vec();
    self.pos += count;
    Ok(bytes)
  }

  /// Read a byte array with length prefix: u32 count + bytes.
  #[inline]
  pub fn read_byte_array(&mut self) -> Result<Vec<u8>> {
    let count = self.read_u32()? as usize;
    self.read_bytes(count)
  }

  // ── Zero-copy methods ────────────────────────────────────────

  /// Read a Bebop string as a borrowed `&str` (zero-copy).
  /// Same wire format as `read_string`: u32 byte_count + UTF-8 bytes + NUL.
  #[inline]
  pub fn read_str(&mut self) -> Result<&'a str> {
    let len = self.read_u32()? as usize;
    // On 32-bit targets usize == u32, so len + 1 can overflow when
    // len == u32::MAX.  Use checked arithmetic to surface a clean error.
    let total = len.checked_add(1).ok_or(DecodeError::UnexpectedEof {
      needed: usize::MAX,
      available: self.remaining(),
    })?;
    self.ensure(total)?; // string bytes + NUL
    let str_bytes = &self.buf[self.pos..self.pos + len];
    self.pos += total; // advance past string bytes + NUL
    validate_utf8(str_bytes)
  }

  /// Read a byte array as a borrowed `&[u8]` (zero-copy).
  /// Same wire format as `read_byte_array`: u32 count + bytes.
  #[inline]
  pub fn read_byte_slice(&mut self) -> Result<&'a [u8]> {
    let count = self.read_u32()? as usize;
    self.read_raw_bytes(count)
  }

  /// Read `count` raw bytes as a borrowed slice (zero-copy).
  /// Like `read_bytes` but borrows instead of allocating.
  #[inline]
  pub fn read_raw_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
    self.ensure(count)?;
    let slice = &self.buf[self.pos..self.pos + count];
    self.pos += count;
    Ok(slice)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::borrow::Cow;
  use alloc::vec;

  #[test]
  fn read_array_malicious_count_returns_eof() {
    // count = 0xFFFFFFFF but buffer only has 100 bytes after the count field
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_u32());
    assert!(result.is_err());
    match result.unwrap_err() {
      DecodeError::UnexpectedEof { .. } => {} // expected
      other => panic!("expected UnexpectedEof, got {:?}", other),
    }
  }

  #[test]
  fn read_array_count_boundary_plus_one_returns_eof() {
    // Buffer has 8 bytes after count field. count = 9 (one more than remaining).
    let mut buf = Vec::new();
    buf.extend_from_slice(&9u32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 8]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_byte());
    assert!(result.is_err());
  }

  #[test]
  fn read_array_count_equals_remaining_succeeds() {
    // Buffer has exactly 3 bytes after count field. count = 3, read 3 bytes.
    let mut buf = Vec::new();
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_array(|r| r.read_byte());
    assert_eq!(result.unwrap(), vec![0xAA, 0xBB, 0xCC]);
  }

  #[test]
  fn read_array_empty_succeeds() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut reader = BebopReader::new(&buf);
    let result: core::result::Result<Vec<u8>, _> = reader.read_array(|r| r.read_byte());
    assert_eq!(result.unwrap(), Vec::<u8>::new());
  }

  #[test]
  fn read_map_malicious_count_returns_eof() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert!(result.is_err());
    match result.unwrap_err() {
      DecodeError::UnexpectedEof { .. } => {}
      other => panic!("expected UnexpectedEof, got {:?}", other),
    }
  }

  #[test]
  fn read_map_count_boundary_plus_one_returns_eof() {
    // Buffer has 4 bytes after count. Minimum entry = 2 bytes, so max count = 2.
    // count = 3 should fail.
    let mut buf = Vec::new();
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert!(result.is_err());
  }

  #[test]
  fn read_map_count_equals_boundary_succeeds() {
    // Buffer has 4 bytes after count. count = 2, each entry = 2 bytes (1 key + 1 value).
    let mut buf = Vec::new();
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&[1, 10, 2, 20]); // key=1,val=10, key=2,val=20
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    let map = result.unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map[&1u8], 10u8);
    assert_eq!(map[&2u8], 20u8);
  }

  #[test]
  fn read_map_empty_succeeds() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_map(|r| {
      let k = r.read_byte()?;
      let v = r.read_byte()?;
      Ok((k, v))
    });
    assert_eq!(result.unwrap().len(), 0);
  }

  #[test]
  fn ensure_overflow_returns_error() {
    let buf = [0u8; 16];
    let mut reader = BebopReader::new(&buf);
    reader.pos = 8;
    let result = reader.skip(usize::MAX);
    assert!(
      result.is_err(),
      "ensure must reject count that overflows pos + count"
    );
  }

  #[test]
  fn read_string_len_overflow_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 16]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_string();
    assert!(result.is_err(), "read_string must reject len=u32::MAX");
  }

  #[test]
  fn read_str_len_overflow_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 16]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_str();
    assert!(result.is_err(), "read_str must reject len=u32::MAX");
  }

  #[test]
  fn read_scalar_array_i32_round_trip() {
    let values: Vec<i32> = vec![1, -2, i32::MAX, i32::MIN, 0];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
  }

  #[test]
  fn read_scalar_array_f64_round_trip() {
    let values: Vec<f64> = vec![1.0, -0.0, f64::INFINITY, f64::NAN, f64::MIN];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<f64>().unwrap();
    for (a, b) in values.iter().zip(result.iter()) {
      assert_eq!(a.to_bits(), b.to_bits());
    }
  }

  #[test]
  fn read_scalar_array_empty() {
    let values: Vec<i64> = vec![];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i64>().unwrap();
    assert!(result.is_empty());
    assert!(matches!(result, Cow::Borrowed(_)));
  }

  #[test]
  fn read_scalar_array_malicious_count_returns_error() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    buf.extend_from_slice(&[0u8; 100]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_scalar_array::<i32>();
    assert!(result.is_err());
  }

  #[test]
  fn read_scalar_array_count_too_large_for_buffer() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&u32::MAX.to_le_bytes());
    buf.extend_from_slice(&[0u8; 64]);
    let mut reader = BebopReader::new(&buf);
    let result = reader.read_scalar_array::<i64>();
    assert!(result.is_err());
  }

  #[test]
  fn read_scalar_array_aligned_returns_borrowed() {
    let values: Vec<i32> = vec![10, 20, 30];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let bytes = writer.into_bytes();
    let mut reader = BebopReader::new(&bytes);
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
    if cfg!(target_endian = "little") {
      assert!(
        matches!(result, Cow::Borrowed(_)),
        "expected Cow::Borrowed on LE-aligned buffer"
      );
    }
  }

  #[test]
  fn read_scalar_array_unaligned_returns_owned() {
    let values: Vec<i32> = vec![42, 99];
    let mut writer = crate::BebopWriter::new();
    writer.write_scalar_array(&values);
    let aligned_bytes = writer.into_bytes();
    // Prepend 1 byte to misalign the data
    let mut buf = vec![0u8];
    buf.extend_from_slice(&aligned_bytes);
    let mut reader = BebopReader::new(&buf[1..]);
    let result = reader.read_scalar_array::<i32>().unwrap();
    assert_eq!(&*result, &*values);
    if cfg!(target_endian = "little") {
      assert!(
        matches!(result, Cow::Owned(_)),
        "expected Cow::Owned on unaligned buffer"
      );
    }
  }
}
