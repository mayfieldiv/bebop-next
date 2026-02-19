#[cfg(feature = "alloc")]
use crate::HashMap;
#[cfg(feature = "alloc")]
use core::hash::BuildHasher;
use core::mem::size_of;

/// Wire size of a u32 length prefix.
pub const WIRE_LEN_PREFIX_SIZE: usize = size_of::<u32>();
/// Wire size of a message/field tag byte.
pub const WIRE_TAG_SIZE: usize = size_of::<u8>();
/// Wire size of the NUL terminator appended to strings.
pub const WIRE_NUL_TERMINATOR_SIZE: usize = size_of::<u8>();
/// Wire size of a message end marker tag.
pub const WIRE_MESSAGE_END_MARKER_SIZE: usize = WIRE_TAG_SIZE;
/// Base message overhead: length prefix + end marker.
pub const WIRE_MESSAGE_BASE_SIZE: usize = WIRE_LEN_PREFIX_SIZE + WIRE_MESSAGE_END_MARKER_SIZE;

#[inline]
pub const fn string_size(len: usize) -> usize {
  WIRE_LEN_PREFIX_SIZE + len + WIRE_NUL_TERMINATOR_SIZE
}

#[inline]
pub const fn byte_array_size(len: usize) -> usize {
  WIRE_LEN_PREFIX_SIZE + len
}

#[inline]
pub fn array_size<T>(items: &[T], mut elem_size: impl FnMut(&T) -> usize) -> usize {
  WIRE_LEN_PREFIX_SIZE + items.iter().map(&mut elem_size).sum::<usize>()
}

#[inline]
#[cfg(feature = "alloc")]
pub fn map_size<K, V, S>(
  map: &HashMap<K, V, S>,
  mut entry_size: impl FnMut(&K, &V) -> usize,
) -> usize
where
  S: BuildHasher,
{
  WIRE_LEN_PREFIX_SIZE + map.iter().map(|(k, v)| entry_size(k, v)).sum::<usize>()
}

#[inline]
pub const fn tagged_size(payload_size: usize) -> usize {
  WIRE_TAG_SIZE + payload_size
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn string_size_empty_and_non_empty() {
    assert_eq!(
      string_size(0),
      WIRE_LEN_PREFIX_SIZE + WIRE_NUL_TERMINATOR_SIZE
    );
    assert_eq!(
      string_size(5),
      WIRE_LEN_PREFIX_SIZE + 5 + WIRE_NUL_TERMINATOR_SIZE
    );
  }

  #[test]
  fn byte_array_size_is_prefix_plus_payload() {
    assert_eq!(byte_array_size(0), WIRE_LEN_PREFIX_SIZE);
    assert_eq!(byte_array_size(12), WIRE_LEN_PREFIX_SIZE + 12);
  }

  #[test]
  fn tagged_size_is_tag_plus_payload() {
    assert_eq!(tagged_size(0), WIRE_TAG_SIZE);
    assert_eq!(tagged_size(9), WIRE_TAG_SIZE + 9);
  }

  #[test]
  fn array_size_fixed_and_variable() {
    let fixed = [10u32, 20u32, 30u32];
    let fixed_sz = array_size(&fixed, |_| size_of::<u32>());
    assert_eq!(
      fixed_sz,
      WIRE_LEN_PREFIX_SIZE + fixed.len() * size_of::<u32>()
    );

    let variable = ["a", "xyz"];
    let var_sz = array_size(&variable, |s| string_size(s.len()));
    assert_eq!(
      var_sz,
      WIRE_LEN_PREFIX_SIZE + string_size("a".len()) + string_size("xyz".len())
    );
  }

  #[test]
  #[cfg(feature = "alloc")]
  fn map_size_aggregates_entries() {
    let mut m = HashMap::new();
    m.insert("k1", "v1");
    m.insert("k2", "value2");
    let computed = map_size(&m, |k, v| string_size(k.len()) + string_size(v.len()));
    let expected = WIRE_LEN_PREFIX_SIZE
      + m
        .iter()
        .map(|(k, v)| string_size(k.len()) + string_size(v.len()))
        .sum::<usize>();
    assert_eq!(computed, expected);
  }
}
