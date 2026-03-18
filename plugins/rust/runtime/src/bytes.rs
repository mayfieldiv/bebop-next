use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

/// A newtype around `Cow<'buf, [u8]>` that provides correct serde serialization
/// via `serde_bytes` in all nesting contexts.
///
/// Without this newtype, `Cow<[u8]>` inside `Option`, `Vec`, or `HashMap` would
/// either fail to compile (due to `#[serde(with = ...)]` not composing with
/// `Option`) or serialize each byte as a separate number in JSON.
///
/// `BebopBytes` carries its own `Serialize`/`Deserialize` impls, so it works
/// automatically in any context: direct fields, `Option<BebopBytes>`,
/// `Vec<BebopBytes>`, `HashMap<K, BebopBytes>`, etc.
#[derive(Clone, Default)]
pub struct BebopBytes<'buf>(pub Cow<'buf, [u8]>);

impl<'buf> BebopBytes<'buf> {
  /// Create a `BebopBytes` borrowing from a decode buffer (zero-copy).
  #[inline]
  pub fn borrowed(s: &'buf [u8]) -> Self {
    BebopBytes(Cow::Borrowed(s))
  }

  /// Convert to a `'static` lifetime by cloning the data if borrowed.
  #[inline]
  pub fn into_owned(self) -> BebopBytes<'static> {
    BebopBytes(Cow::Owned(self.0.into_owned()))
  }
}

impl<'buf> Deref for BebopBytes<'buf> {
  type Target = [u8];

  #[inline]
  fn deref(&self) -> &[u8] {
    &self.0
  }
}

impl<'buf> AsRef<[u8]> for BebopBytes<'buf> {
  #[inline]
  fn as_ref(&self) -> &[u8] {
    &self.0
  }
}

impl<'buf> fmt::Debug for BebopBytes<'buf> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("BebopBytes").field(&self.0).finish()
  }
}

impl<'buf> PartialEq for BebopBytes<'buf> {
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl<'buf> Eq for BebopBytes<'buf> {}

impl<'buf> Hash for BebopBytes<'buf> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl From<Vec<u8>> for BebopBytes<'static> {
  #[inline]
  fn from(v: Vec<u8>) -> Self {
    BebopBytes(Cow::Owned(v))
  }
}

impl<'buf> From<&'buf [u8]> for BebopBytes<'buf> {
  #[inline]
  fn from(s: &'buf [u8]) -> Self {
    BebopBytes(Cow::Borrowed(s))
  }
}

impl<'buf> From<Cow<'buf, [u8]>> for BebopBytes<'buf> {
  #[inline]
  fn from(c: Cow<'buf, [u8]>) -> Self {
    BebopBytes(c)
  }
}

#[cfg(feature = "serde")]
impl<'buf> serde::Serialize for BebopBytes<'buf> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serde_bytes::serialize(self.0.as_ref(), serializer)
  }
}

#[cfg(feature = "serde")]
impl<'de, 'buf> serde::Deserialize<'de> for BebopBytes<'buf> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let bytes = serde_bytes::ByteBuf::deserialize(deserializer)?;
    Ok(BebopBytes(Cow::Owned(bytes.into_vec())))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::vec;

  #[test]
  fn default_is_empty() {
    let b = BebopBytes::default();
    assert!(b.is_empty());
    assert_eq!(&*b, &[] as &[u8]);
  }

  #[test]
  fn from_vec() {
    let b = BebopBytes::from(vec![1, 2, 3]);
    assert_eq!(&*b, &[1, 2, 3]);
  }

  #[test]
  fn from_slice() {
    let data = [4, 5, 6];
    let b = BebopBytes::from(&data[..]);
    assert_eq!(&*b, &[4, 5, 6]);
  }

  #[test]
  fn borrowed() {
    let data = [7, 8, 9];
    let b = BebopBytes::borrowed(&data);
    assert_eq!(&*b, &[7, 8, 9]);
  }

  #[test]
  fn into_owned_round_trip() {
    let data = [10, 11, 12];
    let b = BebopBytes::borrowed(&data);
    let owned: BebopBytes<'static> = b.into_owned();
    assert_eq!(&*owned, &[10, 11, 12]);
  }

  #[test]
  fn eq_and_hash() {
    let a = BebopBytes::from(vec![1, 2, 3]);
    let b = BebopBytes::from(vec![1, 2, 3]);
    let c = BebopBytes::from(vec![4, 5, 6]);
    assert_eq!(a, b);
    assert_ne!(a, c);

    // Hash consistency: equal values produce equal hashes
    let mut map = crate::HashMap::new();
    map.insert(a, "found");
    assert_eq!(map.get(&b), Some(&"found"));
    assert_eq!(map.get(&c), None);
  }

  #[test]
  fn deref_and_as_ref() {
    let b = BebopBytes::from(vec![1, 2]);
    let _slice: &[u8] = &b;
    let _slice2: &[u8] = b.as_ref();
    assert_eq!(_slice, &[1, 2]);
    assert_eq!(_slice2, &[1, 2]);
  }

  #[test]
  fn clone() {
    let b = BebopBytes::from(vec![1, 2, 3]);
    let c = b.clone();
    assert_eq!(b, c);
  }
}
