use alloc::vec::Vec;
use core::hash::Hash;

use crate::temporal::{BebopDuration, BebopTimestamp};
use crate::traits::FixedScalar;
use crate::HashMap;
use crate::{bf16, f16};

/// Accumulating writer for Bebop wire format.
///
/// All multi-byte integers are little-endian.
/// Strings are: u32 length + UTF-8 bytes + NUL byte.
/// Messages are: u32 body_length + tagged fields + 0x00 end marker.
pub struct BebopWriter {
  buf: Vec<u8>,
}

impl Default for BebopWriter {
  fn default() -> Self {
    Self::new()
  }
}

impl BebopWriter {
  pub fn new() -> Self {
    Self { buf: Vec::new() }
  }

  pub fn with_capacity(cap: usize) -> Self {
    Self {
      buf: Vec::with_capacity(cap),
    }
  }

  pub fn into_bytes(self) -> Vec<u8> {
    self.buf
  }

  pub fn len(&self) -> usize {
    self.buf.len()
  }

  pub fn is_empty(&self) -> bool {
    self.buf.is_empty()
  }

  // ── Primitives ──────────────────────────────────────────────

  pub fn write_byte(&mut self, v: u8) {
    self.buf.push(v);
  }

  pub fn write_bool(&mut self, v: bool) {
    self.buf.push(if v { 1 } else { 0 });
  }

  pub fn write_i8(&mut self, v: i8) {
    self.buf.push(v as u8);
  }

  pub fn write_u16(&mut self, v: u16) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_i16(&mut self, v: i16) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_u32(&mut self, v: u32) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_i32(&mut self, v: i32) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_u64(&mut self, v: u64) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_i64(&mut self, v: i64) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_i128(&mut self, v: i128) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_u128(&mut self, v: u128) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_f16(&mut self, v: f16) {
    self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
  }

  pub fn write_bf16(&mut self, v: bf16) {
    self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
  }

  pub fn write_f32(&mut self, v: f32) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  pub fn write_f64(&mut self, v: f64) {
    self.buf.extend_from_slice(&v.to_le_bytes());
  }

  // ── String ──────────────────────────────────────────────────

  /// Write a Bebop string: u32 byte_count + UTF-8 bytes + NUL.
  /// The u32 is the number of UTF-8 bytes (NOT including the trailing NUL).
  pub fn write_string(&mut self, s: &str) {
    let len = s.len() as u32;
    self.write_u32(len);
    self.buf.extend_from_slice(s.as_bytes());
    self.buf.push(0); // NUL terminator
  }

  // ── UUID ────────────────────────────────────────────────────

  pub fn write_uuid(&mut self, v: uuid::Uuid) {
    self.buf.extend_from_slice(v.as_bytes());
  }

  // ── Timestamp / Duration ────────────────────────────────────

  pub fn write_timestamp(&mut self, v: BebopTimestamp) {
    self.write_i64(v.seconds);
    self.write_i32(v.nanos);
  }

  pub fn write_duration(&mut self, v: BebopDuration) {
    self.write_i64(v.seconds);
    self.write_i32(v.nanos);
  }

  // ── Message helpers ─────────────────────────────────────────

  /// Reserve space for a message length prefix. Returns the position to fill later.
  pub fn reserve_message_length(&mut self) -> usize {
    let pos = self.buf.len();
    self.buf.extend_from_slice(&[0u8; 4]);
    pos
  }

  /// Fill in the message body length at the previously reserved position.
  /// The length is (current position - reserved position - 4 bytes for the length itself).
  pub fn fill_message_length(&mut self, pos: usize) {
    let body_len = (self.buf.len() - pos - 4) as u32;
    self.buf[pos..pos + 4].copy_from_slice(&body_len.to_le_bytes());
  }

  /// Write a message field tag.
  pub fn write_tag(&mut self, tag: u8) {
    self.buf.push(tag);
  }

  /// Write the message end marker (tag 0).
  pub fn write_end_marker(&mut self) {
    self.buf.push(0);
  }

  // ── Raw bytes ──────────────────────────────────────────────

  /// Write raw bytes without a length prefix.
  pub fn write_raw(&mut self, v: &[u8]) {
    self.buf.extend_from_slice(v);
  }

  // ── Collections ─────────────────────────────────────────────

  /// Write an array: u32 count + elements.
  pub fn write_array<T>(&mut self, items: &[T], mut write_elem: impl FnMut(&mut Self, &T)) {
    self.write_u32(items.len() as u32);
    for item in items {
      write_elem(self, item);
    }
  }

  /// Write a map: u32 count + (key, value) pairs.
  pub fn write_map<K: Eq + Hash, V>(
    &mut self,
    map: &HashMap<K, V>,
    mut write_entry: impl FnMut(&mut Self, &K, &V),
  ) {
    self.write_u32(map.len() as u32);
    for (k, v) in map {
      write_entry(self, k, v);
    }
  }

  /// Write a byte array with length prefix: u32 count + bytes.
  pub fn write_byte_array(&mut self, v: &[u8]) {
    self.write_u32(v.len() as u32);
    self.buf.extend_from_slice(v);
  }

  /// Write a fixed-size array of any scalar type (no length prefix).
  pub fn write_fixed_array<T: FixedScalar, const N: usize>(&mut self, arr: &[T; N]) {
    for item in arr {
      item.write_to(self);
    }
  }

  /// Write a fixed-size i32 array (no length prefix).
  pub fn write_fixed_i32_array<const N: usize>(&mut self, arr: &[i32; N]) {
    self.write_fixed_array::<i32, N>(arr);
  }
}
