use alloc::borrow::Cow;

use serde::{Deserialize, Deserializer, Serializer};

/// Serde adapter for `Cow<[u8]>`.
///
/// Serialization uses `serde_bytes` for compact binary representations in
/// non-human-readable formats. Deserialization returns an owned buffer.
pub fn serialize<S, B>(bytes: &B, serializer: S) -> Result<S::Ok, S::Error>
where
  B: AsRef<[u8]> + ?Sized,
  S: Serializer,
{
  serde_bytes::serialize(bytes.as_ref(), serializer)
}

pub fn deserialize<'de: 'buf, 'buf, D>(deserializer: D) -> Result<Cow<'buf, [u8]>, D::Error>
where
  D: Deserializer<'de>,
{
  let bytes = serde_bytes::ByteBuf::deserialize(deserializer)?;
  Ok(Cow::Owned(bytes.into_vec()))
}
