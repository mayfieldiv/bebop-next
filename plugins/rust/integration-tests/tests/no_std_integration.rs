#![cfg(feature = "alloc-map")]

use bebop_runtime::HashMap;
use bebop_runtime::{BebopDecode, BebopEncode};

use bebop_integration_tests::no_std_types::*;

#[test]
fn no_std_position_round_trip() {
  let p = Position::new(1.25, -2.5);
  let bytes = p.to_bytes();
  let p2 = Position::from_bytes(&bytes).unwrap();
  assert_eq!(p2.x, 1.25);
  assert_eq!(p2.y, -2.5);
  assert_eq!(p.encoded_size(), bytes.len());
}

#[test]
fn no_std_label_round_trip() {
  let l = Label::new("hello");
  let bytes = l.to_bytes();
  let l2 = Label::from_bytes(&bytes).unwrap();
  assert_eq!(l2.text, "hello");
  assert_eq!(l.encoded_size(), bytes.len());
}

#[test]
fn no_std_blob_round_trip() {
  let b = Blob::new(&[1u8, 2, 3, 4][..]);
  let bytes = b.to_bytes();
  let b2 = Blob::from_bytes(&bytes).unwrap();
  assert_eq!(b2.data.as_ref(), &[1u8, 2, 3, 4]);
  assert_eq!(b.encoded_size(), bytes.len());
}

#[test]
fn no_std_snapshot_round_trip() {
  let points = vec![Position::new(0.0, 1.0), Position::new(2.0, 3.0)];
  let mut snapshot = Snapshot::default();
  snapshot.id = Some(7);
  snapshot.title = Some("demo".into());
  snapshot.points = Some(points);
  snapshot.payload = Some((&[9u8, 8, 7][..]).into());
  let mut counts = HashMap::new();
  counts.insert("alpha".into(), 1u32);
  counts.insert("beta".into(), 2u32);
  snapshot.counts = Some(counts);

  let bytes = snapshot.to_bytes();
  let decoded = Snapshot::from_bytes(&bytes).unwrap();

  assert_eq!(decoded.id, Some(7));
  assert_eq!(decoded.title.as_deref(), Some("demo"));
  assert_eq!(decoded.points.as_ref().map(|v| v.len()), Some(2));
  assert_eq!(decoded.payload.as_deref(), Some(&[9u8, 8, 7][..]));
  assert_eq!(decoded.counts.as_ref().map(|m| m.len()), Some(2));
  assert_eq!(
    decoded
      .counts
      .as_ref()
      .and_then(|m| m.get("alpha"))
      .copied(),
    Some(1)
  );
  assert_eq!(
    decoded.counts.as_ref().and_then(|m| m.get("beta")).copied(),
    Some(2)
  );
  assert_eq!(snapshot.encoded_size(), bytes.len());
}
