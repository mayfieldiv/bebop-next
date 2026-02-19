use std::collections::HashMap;
use std::hash::Hash;

use crate::{DecodeError, bf16, f16};

type Result<T> = std::result::Result<T, DecodeError>;

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
  pub fn new(buf: &'a [u8]) -> Self {
    Self { buf, pos: 0 }
  }

  pub fn position(&self) -> usize {
    self.pos
  }

  pub fn remaining(&self) -> usize {
    self.buf.len().saturating_sub(self.pos)
  }

  fn ensure(&self, count: usize) -> Result<()> {
    if self.pos + count > self.buf.len() {
      Err(DecodeError::UnexpectedEof {
        needed: count,
        available: self.remaining(),
      })
    } else {
      Ok(())
    }
  }

  fn advance(&mut self, count: usize) -> &'a [u8] {
    let slice = &self.buf[self.pos..self.pos + count];
    self.pos += count;
    slice
  }

  // ── Primitives ──────────────────────────────────────────────

  pub fn read_bool(&mut self) -> Result<bool> {
    self.ensure(1)?;
    let v = self.buf[self.pos];
    self.pos += 1;
    Ok(v != 0)
  }

  pub fn read_byte(&mut self) -> Result<u8> {
    self.ensure(1)?;
    let v = self.buf[self.pos];
    self.pos += 1;
    Ok(v)
  }

  pub fn read_i8(&mut self) -> Result<i8> {
    Ok(self.read_byte()? as i8)
  }

  pub fn read_u16(&mut self) -> Result<u16> {
    self.ensure(2)?;
    let bytes: [u8; 2] = self.advance(2).try_into().unwrap();
    Ok(u16::from_le_bytes(bytes))
  }

  pub fn read_i16(&mut self) -> Result<i16> {
    self.ensure(2)?;
    let bytes: [u8; 2] = self.advance(2).try_into().unwrap();
    Ok(i16::from_le_bytes(bytes))
  }

  pub fn read_u32(&mut self) -> Result<u32> {
    self.ensure(4)?;
    let bytes: [u8; 4] = self.advance(4).try_into().unwrap();
    Ok(u32::from_le_bytes(bytes))
  }

  pub fn read_i32(&mut self) -> Result<i32> {
    self.ensure(4)?;
    let bytes: [u8; 4] = self.advance(4).try_into().unwrap();
    Ok(i32::from_le_bytes(bytes))
  }

  pub fn read_u64(&mut self) -> Result<u64> {
    self.ensure(8)?;
    let bytes: [u8; 8] = self.advance(8).try_into().unwrap();
    Ok(u64::from_le_bytes(bytes))
  }

  pub fn read_i64(&mut self) -> Result<i64> {
    self.ensure(8)?;
    let bytes: [u8; 8] = self.advance(8).try_into().unwrap();
    Ok(i64::from_le_bytes(bytes))
  }

  pub fn read_i128(&mut self) -> Result<i128> {
    self.ensure(16)?;
    let bytes: [u8; 16] = self.advance(16).try_into().unwrap();
    Ok(i128::from_le_bytes(bytes))
  }

  pub fn read_u128(&mut self) -> Result<u128> {
    self.ensure(16)?;
    let bytes: [u8; 16] = self.advance(16).try_into().unwrap();
    Ok(u128::from_le_bytes(bytes))
  }

  pub fn read_f16(&mut self) -> Result<f16> {
    Ok(f16::from_bits(self.read_u16()?))
  }

  pub fn read_bf16(&mut self) -> Result<bf16> {
    Ok(bf16::from_bits(self.read_u16()?))
  }

  pub fn read_f32(&mut self) -> Result<f32> {
    self.ensure(4)?;
    let bytes: [u8; 4] = self.advance(4).try_into().unwrap();
    Ok(f32::from_le_bytes(bytes))
  }

  pub fn read_f64(&mut self) -> Result<f64> {
    self.ensure(8)?;
    let bytes: [u8; 8] = self.advance(8).try_into().unwrap();
    Ok(f64::from_le_bytes(bytes))
  }

  // ── String ──────────────────────────────────────────────────

  /// Read a Bebop string: u32 byte_count + UTF-8 bytes + NUL terminator.
  /// The u32 is the number of UTF-8 bytes (NOT including the trailing NUL).
  pub fn read_string(&mut self) -> Result<String> {
    let len = self.read_u32()? as usize;
    self.ensure(len + 1)?; // string bytes + NUL
    let str_bytes = &self.buf[self.pos..self.pos + len];
    self.pos += len + 1; // advance past string bytes + NUL
    String::from_utf8(str_bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
  }

  // ── UUID ────────────────────────────────────────────────────

  pub fn read_uuid(&mut self) -> Result<[u8; 16]> {
    self.ensure(16)?;
    let bytes: [u8; 16] = self.advance(16).try_into().unwrap();
    Ok(bytes)
  }

  // ── Timestamp / Duration ────────────────────────────────────

  /// Read a timestamp: i64 seconds + i32 nanos (12 bytes total).
  pub fn read_timestamp(&mut self) -> Result<(i64, i32)> {
    let seconds = self.read_i64()?;
    let nanos = self.read_i32()?;
    Ok((seconds, nanos))
  }

  /// Read a duration: i64 seconds + i32 nanos (12 bytes total).
  pub fn read_duration(&mut self) -> Result<(i64, i32)> {
    let seconds = self.read_i64()?;
    let nanos = self.read_i32()?;
    Ok((seconds, nanos))
  }

  // ── Message helpers ─────────────────────────────────────────

  /// Read message body length (u32).
  pub fn read_message_length(&mut self) -> Result<u32> {
    self.read_u32()
  }

  /// Read a message field tag (u8). Tag 0 = end marker.
  pub fn read_tag(&mut self) -> Result<u8> {
    self.read_byte()
  }

  /// Skip `count` bytes.
  pub fn skip(&mut self, count: usize) -> Result<()> {
    self.ensure(count)?;
    self.pos += count;
    Ok(())
  }

  // ── Collections ─────────────────────────────────────────────

  /// Read a dynamic array: u32 count + elements.
  pub fn read_array<T>(
    &mut self,
    mut read_elem: impl FnMut(&mut Self) -> Result<T>,
  ) -> Result<Vec<T>> {
    let count = self.read_u32()? as usize;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
      items.push(read_elem(self)?);
    }
    Ok(items)
  }

  /// Read a dynamic map: u32 count + (key, value) pairs.
  pub fn read_map<K: Eq + Hash, V>(
    &mut self,
    mut read_entry: impl FnMut(&mut Self) -> Result<(K, V)>,
  ) -> Result<HashMap<K, V>> {
    let count = self.read_u32()? as usize;
    let mut map = HashMap::with_capacity(count);
    for _ in 0..count {
      let (k, v) = read_entry(self)?;
      map.insert(k, v);
    }
    Ok(map)
  }

  /// Read a fixed-size i32 array (no length prefix).
  pub fn read_fixed_i32_array<const N: usize>(&mut self) -> Result<[i32; N]> {
    let mut arr = [0i32; N];
    for item in &mut arr {
      *item = self.read_i32()?;
    }
    Ok(arr)
  }

  /// Read raw bytes.
  pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
    self.ensure(count)?;
    let bytes = self.buf[self.pos..self.pos + count].to_vec();
    self.pos += count;
    Ok(bytes)
  }

  /// Read a byte array with length prefix: u32 count + bytes.
  pub fn read_byte_array(&mut self) -> Result<Vec<u8>> {
    let count = self.read_u32()? as usize;
    self.read_bytes(count)
  }

  // ── Zero-copy methods ────────────────────────────────────────

  /// Read a Bebop string as a borrowed `&str` (zero-copy).
  /// Same wire format as `read_string`: u32 byte_count + UTF-8 bytes + NUL.
  pub fn read_str(&mut self) -> Result<&'a str> {
    let len = self.read_u32()? as usize;
    self.ensure(len + 1)?;
    let str_bytes = &self.buf[self.pos..self.pos + len];
    self.pos += len + 1; // advance past string bytes + NUL
    std::str::from_utf8(str_bytes).map_err(|_| DecodeError::InvalidUtf8)
  }

  /// Read a byte array as a borrowed `&[u8]` (zero-copy).
  /// Same wire format as `read_byte_array`: u32 count + bytes.
  pub fn read_byte_slice(&mut self) -> Result<&'a [u8]> {
    let count = self.read_u32()? as usize;
    self.read_raw_bytes(count)
  }

  /// Read `count` raw bytes as a borrowed slice (zero-copy).
  /// Like `read_bytes` but borrows instead of allocating.
  pub fn read_raw_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
    self.ensure(count)?;
    let slice = &self.buf[self.pos..self.pos + count];
    self.pos += count;
    Ok(slice)
  }
}
