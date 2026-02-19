#![cfg(not(feature = "std"))]
#![no_std]
#![no_implicit_prelude]

extern crate alloc;
extern crate bebop_integration_tests;
extern crate bebop_runtime;

use ::alloc::vec;
use ::bebop_runtime::HashMap;
use ::bebop_runtime::{BebopDecode, BebopEncode};
use ::core::assert_eq;
use ::core::convert::{AsRef, Into};
use ::core::default::Default;
use ::core::option::Option::Some;

use ::bebop_integration_tests::test_types::*;

#[test]
fn no_std_point_round_trip() {
  let p = Point::new(1.25, -2.5);
  let bytes = p.to_bytes();
  let p2 = Point::from_bytes(&bytes).unwrap();
  assert_eq!(p2.x, 1.25);
  assert_eq!(p2.y, -2.5);
  assert_eq!(p.encoded_size(), bytes.len());
}

#[test]
fn no_std_person_round_trip() {
  let p = Person::new("hello", 42);
  let bytes = p.to_bytes();
  let decoded = Person::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.name, "hello");
  assert_eq!(decoded.age, 42);
  assert_eq!(p.encoded_size(), bytes.len());
}

#[test]
fn no_std_binary_payload_round_trip() {
  let b = BinaryPayload::new(9, &[1u8, 2, 3, 4][..]);
  let bytes = b.to_bytes();
  let decoded = BinaryPayload::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.tag, 9);
  assert_eq!(decoded.data.as_ref(), &[1u8, 2, 3, 4]);
  assert_eq!(b.encoded_size(), bytes.len());
}

#[test]
fn no_std_user_profile_round_trip() {
  let mut profile = UserProfile::default();
  profile.display_name = Some("alice".into());
  profile.email = Some("alice@example.com".into());
  profile.age = Some(30);
  profile.active = Some(true);
  profile.tags = Some(vec!["rust".into(), "bebop".into()]);
  let mut metadata = HashMap::new();
  metadata.insert("role".into(), "admin".into());
  metadata.insert("team".into(), "sdk".into());
  profile.metadata = Some(metadata);
  profile.permissions = Some(Permissions::READ | Permissions::WRITE);

  let bytes = profile.to_bytes();
  let decoded = UserProfile::from_bytes(&bytes).unwrap();

  assert_eq!(decoded.display_name.as_deref(), Some("alice"));
  assert_eq!(decoded.email.as_deref(), Some("alice@example.com"));
  assert_eq!(decoded.age, Some(30));
  assert_eq!(decoded.active, Some(true));
  assert_eq!(decoded.tags.as_ref().map(|t| t.len()), Some(2));
  assert_eq!(decoded.metadata.as_ref().map(|m| m.len()), Some(2));
  assert_eq!(
    decoded
      .metadata
      .as_ref()
      .and_then(|m| m.get("role"))
      .map(|v| v.as_ref()),
    Some("admin")
  );
  assert_eq!(
    decoded
      .metadata
      .as_ref()
      .and_then(|m| m.get("team"))
      .map(|v| v.as_ref()),
    Some("sdk")
  );
  assert_eq!(
    decoded.permissions,
    Some(Permissions::READ | Permissions::WRITE)
  );
  assert_eq!(profile.encoded_size(), bytes.len());
}
