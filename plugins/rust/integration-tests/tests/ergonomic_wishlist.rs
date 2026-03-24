//! Ergonomic API wishlist — these are the call patterns we WANT to work.
//! Currently some of these don't compile. This file drives the design of issue #8.
//!
//! Each test shows the "ugly" current code (commented out) and the desired form.
//!
//! Gated behind "wishlist" feature so it doesn't break normal builds.
//! Run with: cargo test --test ergonomic_wishlist --features std,wishlist

#![allow(unexpected_cfgs)]
#![cfg(feature = "wishlist")]

use bebop_runtime::{BebopBytes, BebopDecode, BebopEncode, BebopFlags, HashMap, Uuid};

use bebop_integration_tests::test_types::*;

// ═══════════════════════════════════════════════════════════════
// Message with_* builder methods
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_builder_scalars() {
  // CURRENT (ugly):
  //   let mut cmd = DrawCommand::default();
  //   cmd.target = Some(Point::new(5.0, 10.0));
  //   cmd.color = Some(Color::Blue);
  //   cmd.thickness = Some(2.5);

  let cmd = DrawCommand::default()
    .with_target(Point::new(5.0, 10.0))
    .with_color(Color::Blue)
    .with_thickness(2.5);

  let bytes = cmd.to_bytes();
  let cmd2 = DrawCommand::from_bytes(&bytes).unwrap();
  assert_eq!(cmd2.target.unwrap().x, 5.0);
  assert_eq!(cmd2.color, Some(Color::Blue));
  assert_eq!(cmd2.thickness, Some(2.5));
}

#[test]
fn message_builder_string_from_str() {
  // CURRENT (ugly):
  //   let mut cmd = DrawCommand::default();
  //   cmd.label = Some(Cow::Owned("test label".to_string()));

  let cmd = DrawCommand::default().with_label("test label");

  assert_eq!(cmd.label.as_deref(), Some("test label"));
}

#[test]
fn message_builder_string_from_string() {
  let owned_string = String::from("dynamic label");
  let cmd = DrawCommand::default().with_label(owned_string);

  assert_eq!(cmd.label.as_deref(), Some("dynamic label"));
}

#[test]
fn message_builder_string_array_from_str_array() {
  // CURRENT (ugly):
  //   profile.tags = Some(vec![
  //     Cow::Owned("rust".to_string()),
  //     Cow::Owned("bebop".to_string()),
  //   ]);

  let profile = UserProfile::default().with_tags(["rust", "bebop"]);

  let tags = profile.tags.as_ref().unwrap();
  assert_eq!(tags.len(), 2);
  assert_eq!(tags[0], "rust");
  assert_eq!(tags[1], "bebop");
}

#[test]
fn message_builder_string_array_from_vec_string() {
  let tag_vec = vec!["alpha".to_string(), "beta".to_string()];
  let profile = UserProfile::default().with_tags(tag_vec);

  let tags = profile.tags.as_ref().unwrap();
  assert_eq!(tags[0], "alpha");
  assert_eq!(tags[1], "beta");
}

#[test]
fn message_builder_string_map_from_tuple_array() {
  // CURRENT (ugly):
  //   let mut meta = HashMap::new();
  //   meta.insert(Cow::Owned("theme".to_string()), Cow::Owned("dark".to_string()));
  //   meta.insert(Cow::Owned("lang".to_string()), Cow::Owned("en".to_string()));
  //   profile.metadata = Some(meta);

  let profile = UserProfile::default().with_metadata([("theme", "dark"), ("lang", "en")]);

  let meta = profile.metadata.as_ref().unwrap();
  assert_eq!(meta["theme"], "dark");
  assert_eq!(meta["lang"], "en");
}

#[test]
fn message_builder_string_map_from_hashmap() {
  let mut map = HashMap::new();
  map.insert("key".to_string(), "val".to_string());

  let profile = UserProfile::default().with_metadata(map);

  let meta = profile.metadata.as_ref().unwrap();
  assert_eq!(meta.len(), 1);
}

#[test]
fn message_builder_map_string_uint32_from_tuples() {
  // CURRENT (ugly):
  //   let mut items = HashMap::new();
  //   items.insert(Cow::Owned("sword".to_string()), 1u32);
  //   items.insert(Cow::Owned("potion".to_string()), 5u32);
  //   inv.items = Some(items);

  let inv = Inventory::default()
    .with_items([("sword", 1u32), ("potion", 5)])
    .with_label("player inventory");

  let items = inv.items.as_ref().unwrap();
  assert_eq!(items.len(), 2);
}

#[test]
fn message_builder_flags_field() {
  // CURRENT (ugly):
  //   profile.permissions = Some(Permissions::READ | Permissions::WRITE);

  let profile = UserProfile::default().with_permissions(Permissions::READ | Permissions::WRITE);

  assert_eq!(
    profile.permissions,
    Some(Permissions::READ | Permissions::WRITE)
  );
}

#[test]
fn message_builder_full_chain() {
  // CURRENT (ugly — 8 lines of mut + field assignment):
  //   let mut profile = UserProfile::default();
  //   profile.display_name = Some(Cow::Owned("Alice".to_string()));
  //   profile.email = Some(Cow::Owned("alice@example.com".to_string()));
  //   profile.age = Some(30);
  //   profile.active = Some(true);
  //   profile.tags = Some(vec![Cow::Owned("admin".to_string())]);
  //   let mut meta = HashMap::new();
  //   meta.insert(Cow::Owned("key".to_string()), Cow::Owned("val".to_string()));
  //   profile.metadata = Some(meta);
  //   profile.permissions = Some(Permissions::READ | Permissions::WRITE);

  let profile = UserProfile::default()
    .with_display_name("Alice")
    .with_email("alice@example.com")
    .with_age(30)
    .with_active(true)
    .with_tags(["admin"])
    .with_metadata([("key", "val")])
    .with_permissions(Permissions::READ | Permissions::WRITE);

  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  assert_eq!(p2.display_name.as_deref(), Some("Alice"));
  assert_eq!(p2.email.as_deref(), Some("alice@example.com"));
  assert_eq!(p2.age, Some(30));
  assert_eq!(p2.active, Some(true));
  assert_eq!(p2.tags.as_ref().unwrap().len(), 1);
  assert_eq!(p2.metadata.as_ref().unwrap().len(), 1);
  assert_eq!(p2.permissions, Some(Permissions::READ | Permissions::WRITE));
}

#[test]
fn message_builder_byte_array() {
  // CURRENT (ugly):
  //   msg.payload = Some(BebopBytes::from(vec![0xDE, 0xAD]));

  let msg = ByteArrayMessage::default()
    .with_label("test")
    .with_payload(vec![0xDE, 0xAD]);

  assert_eq!(msg.label.as_deref(), Some("test"));
  assert_eq!(msg.payload.as_deref(), Some(&[0xDE, 0xAD][..]));
}

#[test]
fn message_builder_byte_array_from_slice() {
  let msg = ByteArrayMessage::default().with_payload(&[0xCA, 0xFE][..]);

  assert_eq!(msg.payload.as_deref(), Some(&[0xCA, 0xFE][..]));
}

#[test]
fn message_builder_byte_collection() {
  // CURRENT (ugly):
  //   msg.matrix = Some(vec![
  //     BebopBytes::from(vec![1, 2]),
  //     BebopBytes::from(vec![3, 4, 5]),
  //   ]);
  //   let mut tagged = HashMap::new();
  //   tagged.insert(Cow::Owned("a".to_string()), BebopBytes::from(vec![10]));
  //   msg.tagged = Some(tagged);

  let msg = ByteCollectionMessage::default()
    .with_matrix([vec![1u8, 2], vec![3, 4, 5]])
    .with_tagged([("a", vec![10u8]), ("b", vec![20, 30])]);

  let matrix = msg.matrix.as_ref().unwrap();
  assert_eq!(&*matrix[0], &[1, 2]);
  assert_eq!(&*matrix[1], &[3, 4, 5]);
}

#[test]
fn message_builder_integer_key_map() {
  // CURRENT (ugly):
  //   let mut labels = HashMap::new();
  //   labels.insert(7u32, Cow::Owned("seven".to_string()));
  //   msg.labels_by_id = Some(labels);

  let msg = IntegerKeyMaps::default()
    .with_labels_by_id([(7u32, "seven"), (42, "forty-two")])
    .with_flags_by_id([(-1i64, false), (1_000_000i64, true)]);

  let labels = msg.labels_by_id.as_ref().unwrap();
  assert_eq!(labels[&7], "seven");
  assert_eq!(labels[&42], "forty-two");
}

#[test]
fn message_builder_round_trip_preserves_data() {
  // Build with builder, encode, decode, verify
  let original = UserProfile::default()
    .with_display_name("Bob")
    .with_age(25)
    .with_tags(["user", "verified"])
    .with_metadata([("role", "member")]);

  let bytes = original.to_bytes();
  let decoded = UserProfile::from_bytes(&bytes).unwrap();

  assert_eq!(decoded.display_name.as_deref(), Some("Bob"));
  assert_eq!(decoded.age, Some(25));
  let tags = decoded.tags.as_ref().unwrap();
  assert_eq!(tags.len(), 2);
  assert_eq!(tags[0], "user");
  assert_eq!(tags[1], "verified");
  let meta = decoded.metadata.as_ref().unwrap();
  assert_eq!(meta["role"], "member");
}

// ═══════════════════════════════════════════════════════════════
// Struct new() with IntoIterator for collection fields
// ═══════════════════════════════════════════════════════════════

#[test]
fn struct_new_string_array_from_str_array() {
  // ByteMatrix::new currently takes Vec<BebopBytes<'static>>
  // We want: ByteMatrix::new([&[1u8, 2], &[3, 4, 5]])
  // But ByteMatrix is byte[][] so let's use a better example

  // ByteTagMap::new currently takes HashMap<String, BebopBytes<'static>>
  // CURRENT (ugly):
  //   let mut entries = HashMap::new();
  //   entries.insert("key1".to_string(), BebopBytes::from(vec![10, 20]));
  //   let btm = ByteTagMap::new(entries);

  let btm = ByteTagMap::new([("key1", vec![10u8, 20]), ("key2", vec![30, 40, 50])]);
  let bytes = btm.to_bytes();
  let btm2 = ByteTagMap::from_bytes(&bytes).unwrap();
  assert_eq!(&*btm2.entries["key1"], &[10, 20]);
  assert_eq!(&*btm2.entries["key2"], &[30, 40, 50]);
}

#[test]
fn struct_new_byte_matrix_from_slice_array() {
  // CURRENT (ugly):
  //   ByteMatrix::new(vec![BebopBytes::from(vec![1, 2, 3]), BebopBytes::from(vec![4, 5])])

  let bm = ByteMatrix::new([vec![1u8, 2, 3], vec![4, 5]]);
  let bytes = bm.to_bytes();
  let bm2 = ByteMatrix::from_bytes(&bytes).unwrap();
  assert_eq!(&*bm2.rows[0], &[1, 2, 3]);
  assert_eq!(&*bm2.rows[1], &[4, 5]);
}

// ═══════════════════════════════════════════════════════════════
// Scene builder — array of defined types
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_builder_scene() {
  // CURRENT (ugly):
  //   let mut scene = Scene::default();
  //   scene.shapes = Some(vec![...]);
  //   scene.background = Some(Color::Blue);
  //   scene.title = Some(Cow::Owned("test scene".to_string()));

  let scene = Scene::default()
    .with_shapes(vec![
      Shape::Point(Point::new(1.0, 2.0)),
      Shape::Label(TextLabel::new(Point::new(3.0, 4.0), "label")),
    ])
    .with_background(Color::Blue)
    .with_title("test scene");

  let bytes = scene.to_bytes();
  let scene2 = Scene::from_bytes(&bytes).unwrap();
  assert_eq!(scene2.shapes.as_ref().unwrap().len(), 2);
  assert_eq!(scene2.background, Some(Color::Blue));
  assert_eq!(scene2.title.as_deref(), Some("test scene"));
}

// ═══════════════════════════════════════════════════════════════
// Deep nested collections builder
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_builder_deep_nested() {
  // CURRENT (ugly — 8 lines):
  //   let mut branch_a = HashMap::new();
  //   branch_a.insert(Cow::Owned("left".to_string()), NestedLeaf::new("alpha"));
  //   let mut nested = HashMap::new();
  //   nested.insert(Cow::Owned("bucket".to_string()), vec![branch_a]);
  //   let mut msg = DeepNestedCollections::default();
  //   msg.nested = Some(nested);

  let msg = DeepNestedCollections::default().with_nested([(
    "bucket",
    vec![HashMap::from([("left".into(), NestedLeaf::new("alpha"))])],
  )]);

  let bytes = msg.to_bytes();
  let decoded = DeepNestedCollections::from_bytes(&bytes).unwrap();
  let nested = decoded.nested.as_ref().unwrap();
  assert_eq!(nested["bucket"][0]["left"].label, "alpha");
}

// ═══════════════════════════════════════════════════════════════
// ScheduleEntry builder — temporal types
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_builder_schedule_entry() {
  use bebop_runtime::{BebopDuration, BebopTimestamp};

  // CURRENT (ugly):
  //   let mut entry = ScheduleEntry::default();
  //   entry.start = Some(BebopTimestamp { seconds: 1_700_000_000, nanos: 0 });
  //   entry.duration = Some(BebopDuration { seconds: 3600, nanos: 0 });
  //   entry.label = Some(Cow::Owned("meeting".to_string()));

  let entry = ScheduleEntry::default()
    .with_start(BebopTimestamp {
      seconds: 1_700_000_000,
      nanos: 0,
    })
    .with_duration(BebopDuration {
      seconds: 3600,
      nanos: 0,
    })
    .with_label("meeting");

  let bytes = entry.to_bytes();
  let entry2 = ScheduleEntry::from_bytes(&bytes).unwrap();
  assert_eq!(entry2.start.unwrap().seconds, 1_700_000_000);
  assert_eq!(entry2.duration.unwrap().seconds, 3600);
  assert_eq!(entry2.label.as_deref(), Some("meeting"));
}
