#![cfg(feature = "std")]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::bool_assert_comparison)]

use std::borrow::Cow;
use std::collections::HashMap;

use bebop_runtime::{
  bf16, f16, BebopDecode, BebopDuration, BebopEncode, BebopFlags, BebopTimestamp, DecodeError, Uuid,
};

use bebop_integration_tests::test_types::*;

// ═══════════════════════════════════════════════════════════════
// Enum
// ═══════════════════════════════════════════════════════════════

#[test]
fn enum_round_trip() {
  let c = Color::Blue;
  let bytes = c.to_bytes();
  let c2 = Color::from_bytes(&bytes).unwrap();
  assert_eq!(c, c2);
}

#[test]
fn enum_fixed_encoded_size() {
  assert_eq!(Color::FIXED_ENCODED_SIZE, 1);
  assert_eq!(Color::Red.encoded_size(), 1);
}

#[test]
fn enum_try_from_valid() {
  let c: Color = 2u8.try_into().unwrap();
  assert_eq!(c, Color::Green);
}

#[test]
fn enum_try_from_invalid() {
  let result: Result<Color, _> = 99u8.try_into();
  assert!(result.is_err());
}

#[test]
fn enum_into_base_type() {
  let v: u8 = Color::Blue.into();
  assert_eq!(v, 3);
}

// ═══════════════════════════════════════════════════════════════
// Consts
// ═══════════════════════════════════════════════════════════════

#[test]
fn const_scalar_values() {
  assert_eq!(P_CASE, 0x12);
  assert_eq!(EXAMPLE_CONST_INT32, -123);
  assert_eq!(EXAMPLE_CONST_UINT64, 0x123ffffffff);
  assert_eq!(EXAMPLE_CONST_FLOAT64, 123.45678e9);
  assert!(!EXAMPLE_CONST_FALSE);
  assert!(EXAMPLE_CONST_TRUE);
  assert_eq!(HTTP_STATUS_OK, 200);
  assert!(FEATURE_FLAG_ENABLED);
}

#[test]
fn const_float_special_values() {
  assert!(EXAMPLE_CONST_INF.is_infinite() && EXAMPLE_CONST_INF.is_sign_positive());
  assert!(EXAMPLE_CONST_NEG_INF.is_infinite() && EXAMPLE_CONST_NEG_INF.is_sign_negative());
  assert!(EXAMPLE_CONST_NAN.is_nan());
}

#[test]
fn const_string_and_uuid_values() {
  assert_eq!(EXAMPLE_CONST_STRING, "hello \"world\"\nwith newlines");
  assert_eq!(
    EXAMPLE_CONST_GUID,
    Uuid::from_bytes([
      0xE2, 0x15, 0xA9, 0x46, 0xB2, 0x6F, 0x45, 0x67, 0xA2, 0x76, 0x13, 0x13, 0x6F, 0x0A, 0x17,
      0x08,
    ])
  );
}

#[test]
fn const_half_values() {
  assert_eq!(EXAMPLE_CONST_F16.to_bits(), f16::from_f64(1.5).to_bits());
  assert_eq!(EXAMPLE_CONST_BF16.to_bits(), bf16::from_f64(1.5).to_bits());
  assert_eq!(
    EXAMPLE_CONST_F16_FROM_INT.to_bits(),
    f16::from_f64(1.0).to_bits()
  );
  assert_eq!(
    EXAMPLE_CONST_BF16_FROM_INT.to_bits(),
    bf16::from_f64(2.0).to_bits()
  );

  assert!(EXAMPLE_CONST_F16_INF.is_infinite() && !EXAMPLE_CONST_F16_INF.is_sign_negative());
  assert!(
    EXAMPLE_CONST_BF16_NEG_INF.is_infinite() && EXAMPLE_CONST_BF16_NEG_INF.is_sign_negative()
  );
  assert!(EXAMPLE_CONST_F16_NAN.is_nan());
  assert!(EXAMPLE_CONST_BF16_NAN.is_nan());
}

// ═══════════════════════════════════════════════════════════════
// Half-precision types
// ═══════════════════════════════════════════════════════════════

#[test]
fn half_precision_scalars_round_trip_preserves_bits() {
  let src = HalfPrecisionScalars::new(f16::from_bits(0x7E01), bf16::from_bits(0x7FC1));
  let bytes = src.to_bytes();
  let decoded = HalfPrecisionScalars::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.f16_val.to_bits(), 0x7E01);
  assert_eq!(decoded.bf16_val.to_bits(), 0x7FC1);
  assert_eq!(src.encoded_size(), bytes.len());
}

#[test]
fn half_precision_arrays_round_trip_preserves_bits() {
  let src = HalfPrecisionArrays::new(
    vec![
      f16::from_bits(0x0000),
      f16::from_bits(0x3C00),
      f16::from_bits(0xFC00),
    ],
    vec![
      bf16::from_bits(0x0000),
      bf16::from_bits(0x3F80),
      bf16::from_bits(0xFF80),
    ],
    [
      f16::from_bits(0x0001),
      f16::from_bits(0x3C00),
      f16::from_bits(0x7C00),
      f16::from_bits(0x7E00),
    ],
    [
      bf16::from_bits(0x0001),
      bf16::from_bits(0x3F80),
      bf16::from_bits(0x7F80),
      bf16::from_bits(0x7FC0),
    ],
  );

  let bytes = src.to_bytes();
  let decoded = HalfPrecisionArrays::from_bytes(&bytes).unwrap();
  assert_eq!(
    decoded
      .f16_dynamic
      .iter()
      .map(|v| v.to_bits())
      .collect::<Vec<_>>(),
    vec![0x0000, 0x3C00, 0xFC00]
  );
  assert_eq!(
    decoded
      .bf16_dynamic
      .iter()
      .map(|v| v.to_bits())
      .collect::<Vec<_>>(),
    vec![0x0000, 0x3F80, 0xFF80]
  );
  assert_eq!(
    decoded.f16_fixed.map(|v| v.to_bits()),
    [0x0001, 0x3C00, 0x7C00, 0x7E00]
  );
  assert_eq!(
    decoded.bf16_fixed.map(|v| v.to_bits()),
    [0x0001, 0x3F80, 0x7F80, 0x7FC0]
  );
  assert_eq!(src.encoded_size(), bytes.len());
}

#[test]
fn half_precision_message_some_and_none_round_trip() {
  let mut msg = HalfPrecisionMessage::default();
  msg.f16_val = Some(f16::from_bits(0x3555));
  msg.bf16_val = Some(bf16::from_bits(0x3FC0));
  msg.f16_arr = Some(vec![f16::from_bits(0x3C00), f16::from_bits(0xBC00)].into());
  msg.bf16_arr = Some(vec![bf16::from_bits(0x3F80), bf16::from_bits(0xBF80)].into());

  let bytes = msg.to_bytes();
  let decoded = HalfPrecisionMessage::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.f16_val.unwrap().to_bits(), 0x3555);
  assert_eq!(decoded.bf16_val.unwrap().to_bits(), 0x3FC0);
  assert_eq!(
    decoded
      .f16_arr
      .unwrap()
      .iter()
      .map(|v| v.to_bits())
      .collect::<Vec<_>>(),
    vec![0x3C00, 0xBC00]
  );
  assert_eq!(
    decoded
      .bf16_arr
      .unwrap()
      .iter()
      .map(|v| v.to_bits())
      .collect::<Vec<_>>(),
    vec![0x3F80, 0xBF80]
  );
  assert_eq!(msg.encoded_size(), bytes.len());

  let empty = HalfPrecisionMessage::default();
  let empty_bytes = empty.to_bytes();
  let decoded_empty = HalfPrecisionMessage::from_bytes(&empty_bytes).unwrap();
  assert!(decoded_empty.f16_val.is_none());
  assert!(decoded_empty.bf16_val.is_none());
  assert!(decoded_empty.f16_arr.is_none());
  assert!(decoded_empty.bf16_arr.is_none());
  assert_eq!(empty.encoded_size(), empty_bytes.len());
}

// ═══════════════════════════════════════════════════════════════
// Flags enum
// ═══════════════════════════════════════════════════════════════

#[test]
fn flags_round_trip() {
  let perms = Permissions::READ | Permissions::WRITE;
  let bytes = perms.to_bytes();
  let perms2 = Permissions::from_bytes(&bytes).unwrap();
  assert_eq!(perms, perms2);
}

#[test]
fn flags_fixed_encoded_size() {
  assert_eq!(Permissions::FIXED_ENCODED_SIZE, 1);
}

#[test]
fn flags_bitwise_operations() {
  let mut perms = Permissions::READ | Permissions::EXECUTE;
  assert!(perms.contains(Permissions::READ));
  assert!(perms.contains(Permissions::EXECUTE));
  assert!(!perms.contains(Permissions::WRITE));
  assert!(perms.intersects(Permissions::READ));

  perms.insert(Permissions::WRITE);
  assert!(perms.contains(Permissions::WRITE));

  perms.remove(Permissions::READ);
  assert!(!perms.contains(Permissions::READ));

  perms.toggle(Permissions::EXECUTE);
  assert!(!perms.contains(Permissions::EXECUTE));
  assert_eq!(perms, Permissions::WRITE);
}

#[test]
fn flags_empty_and_all() {
  assert!(Permissions::empty().is_empty());
  assert!(Permissions::all().is_all());
  assert_eq!(Permissions::all().bits(), 7);
}

#[test]
fn flags_from_bits() {
  assert_eq!(Permissions::from_bits(3), Some(Permissions(3)));
  // 128 is outside the valid flags range (0..7)
  assert_eq!(Permissions::from_bits(128), None);
}

#[test]
fn flags_from_bits_truncate() {
  let p = Permissions::from_bits_truncate(0xFF);
  assert_eq!(p.bits(), 7); // only valid bits kept
}

#[test]
fn flags_set_difference() {
  let all = Permissions::all();
  let minus_read = all - Permissions::READ;
  assert!(!minus_read.contains(Permissions::READ));
  assert!(minus_read.contains(Permissions::WRITE));
  assert!(minus_read.contains(Permissions::EXECUTE));
}

// ═══════════════════════════════════════════════════════════════
// Fixed-size struct
// ═══════════════════════════════════════════════════════════════

#[test]
fn fixed_struct_new_and_round_trip() {
  let p = Point::new(1.5, 2.5);
  let bytes = p.to_bytes();
  let p2 = Point::from_bytes(&bytes).unwrap();
  assert_eq!(p2.x, 1.5);
  assert_eq!(p2.y, 2.5);
}

#[test]
fn fixed_struct_fixed_encoded_size() {
  assert_eq!(Point::FIXED_ENCODED_SIZE, 8);
  assert_eq!(Point::new(0.0, 0.0).encoded_size(), 8);
}

#[test]
fn fixed_struct_encoded_size_matches_actual() {
  let p = Point::new(core::f32::consts::PI, core::f32::consts::E);
  assert_eq!(p.encoded_size(), p.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Fixed-size struct with defined type fields
// ═══════════════════════════════════════════════════════════════

#[test]
fn composite_fixed_struct_round_trip() {
  let px = Pixel::new(Point::new(10.0, 20.0), Color::Green, 255);
  let bytes = px.to_bytes();
  let px2 = Pixel::from_bytes(&bytes).unwrap();
  assert_eq!(px2.position.x, 10.0);
  assert_eq!(px2.position.y, 20.0);
  assert_eq!(px2.color, Color::Green);
  assert_eq!(px2.alpha, 255);
}

#[test]
fn composite_fixed_struct_fixed_encoded_size() {
  // Point(8) + Color(1) + u8(1) = 10
  assert_eq!(Pixel::FIXED_ENCODED_SIZE, 10);
}

#[test]
fn composite_fixed_struct_encoded_size_matches() {
  let px = Pixel::new(Point::new(0.0, 0.0), Color::Red, 128);
  assert_eq!(px.encoded_size(), px.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Variable-size struct with Cow<str>
// ═══════════════════════════════════════════════════════════════

#[test]
fn string_struct_new_and_round_trip() {
  let p = Person::new("Alice", 30);
  let bytes = p.to_bytes();
  let p2 = Person::from_bytes(&bytes).unwrap();
  assert_eq!(p2.name, "Alice");
  assert_eq!(p2.age, 30);
}

#[test]
fn string_struct_zero_copy() {
  let p = Person::new("Bob", 25);
  let bytes = p.to_bytes();
  let decoded = Person::from_bytes(&bytes).unwrap();
  match &decoded.name {
    Cow::Borrowed(_) => {} // zero-copy confirmed
    Cow::Owned(_) => panic!("expected Cow::Borrowed for zero-copy decode"),
  }
}

#[test]
fn string_struct_into_owned() {
  let p = Person::new("Charlie", 40);
  let bytes = p.to_bytes();
  let decoded = Person::from_bytes(&bytes).unwrap();

  // Convert borrowed → owned, extending lifetime to 'static
  let owned: PersonOwned = decoded.into_owned();
  assert_eq!(owned.name, "Charlie");
  assert_eq!(owned.age, 40);

  // Verify it's now owned
  match &owned.name {
    Cow::Owned(_) => {} // into_owned confirmed
    Cow::Borrowed(_) => panic!("expected Cow::Owned after into_owned"),
  }
}

#[test]
fn string_struct_encoded_size_matches() {
  let p = Person::new("Hello, World!", 99);
  assert_eq!(p.encoded_size(), p.to_bytes().len());
}

#[test]
fn string_struct_type_alias() {
  // PersonOwned = Person<'static>
  let p: PersonOwned = Person::new("Static", 1);
  assert_eq!(p.name, "Static");
}

#[test]
fn string_struct_empty_string() {
  let p = Person::new(String::new(), 0);
  let bytes = p.to_bytes();
  let p2 = Person::from_bytes(&bytes).unwrap();
  assert_eq!(p2.name, "");
  assert_eq!(p2.age, 0);
}

// ═══════════════════════════════════════════════════════════════
// Variable-size struct with Cow<[u8]>
// ═══════════════════════════════════════════════════════════════

#[test]
fn byte_array_struct_round_trip() {
  let bp = BinaryPayload::new(42, vec![0xDE, 0xAD, 0xBE, 0xEF]);
  let bytes = bp.to_bytes();
  let bp2 = BinaryPayload::from_bytes(&bytes).unwrap();
  assert_eq!(bp2.tag, 42);
  assert_eq!(&*bp2.data, &[0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn byte_array_struct_zero_copy() {
  let bp = BinaryPayload::new(1, vec![1, 2, 3]);
  let bytes = bp.to_bytes();
  let decoded = BinaryPayload::from_bytes(&bytes).unwrap();
  match &decoded.data {
    Cow::Borrowed(_) => {} // zero-copy confirmed
    Cow::Owned(_) => panic!("expected Cow::Borrowed for zero-copy byte array"),
  }
}

#[test]
fn byte_array_struct_into_owned() {
  let bp = BinaryPayload::new(1, vec![9, 8, 7]);
  let bytes = bp.to_bytes();
  let decoded = BinaryPayload::from_bytes(&bytes).unwrap();
  let owned: BinaryPayloadOwned = decoded.into_owned();
  assert_eq!(&*owned.data, &[9, 8, 7]);
  match &owned.data {
    Cow::Owned(_) => {}
    Cow::Borrowed(_) => panic!("expected Cow::Owned after into_owned"),
  }
}

#[test]
fn byte_array_struct_empty_data() {
  let bp = BinaryPayload::new(0, vec![]);
  let bytes = bp.to_bytes();
  let bp2 = BinaryPayload::from_bytes(&bytes).unwrap();
  assert!(bp2.data.is_empty());
}

#[test]
fn byte_array_struct_encoded_size_matches() {
  let bp = BinaryPayload::new(42, vec![1, 2, 3, 4, 5]);
  assert_eq!(bp.encoded_size(), bp.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Multiple Cow<str> fields
// ═══════════════════════════════════════════════════════════════

#[test]
fn multi_string_struct_round_trip() {
  let addr = Address::new("123 Main St", "Springfield", "US", "62704");
  let bytes = addr.to_bytes();
  let addr2 = Address::from_bytes(&bytes).unwrap();
  assert_eq!(addr2.street, "123 Main St");
  assert_eq!(addr2.city, "Springfield");
  assert_eq!(addr2.country, "US");
  assert_eq!(addr2.zip_code, "62704");
}

#[test]
fn multi_string_struct_all_fields_zero_copy() {
  let addr = Address::new("A", "B", "C", "D");
  let bytes = addr.to_bytes();
  let decoded = Address::from_bytes(&bytes).unwrap();
  assert!(matches!(decoded.street, Cow::Borrowed(_)));
  assert!(matches!(decoded.city, Cow::Borrowed(_)));
  assert!(matches!(decoded.country, Cow::Borrowed(_)));
  assert!(matches!(decoded.zip_code, Cow::Borrowed(_)));
}

#[test]
fn multi_string_struct_into_owned() {
  let addr = Address::new("A", "B", "C", "D");
  let bytes = addr.to_bytes();
  let decoded = Address::from_bytes(&bytes).unwrap();
  let owned: AddressOwned = decoded.into_owned();
  assert!(matches!(owned.street, Cow::Owned(_)));
  assert!(matches!(owned.city, Cow::Owned(_)));
  assert!(matches!(owned.country, Cow::Owned(_)));
  assert!(matches!(owned.zip_code, Cow::Owned(_)));
}

#[test]
fn multi_string_struct_encoded_size_matches() {
  let addr = Address::new("Long Street Name", "City", "Country", "12345");
  assert_eq!(addr.encoded_size(), addr.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Fixed array struct
// ═══════════════════════════════════════════════════════════════

#[test]
fn fixed_array_struct_round_trip() {
  let m = Matrix2x2::new([1.0, 2.0, 3.0, 4.0]);
  let bytes = m.to_bytes();
  let m2 = Matrix2x2::from_bytes(&bytes).unwrap();
  assert_eq!(m2.values, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn fixed_array_struct_fixed_encoded_size() {
  // 4 * f32(4 bytes) = 16
  assert_eq!(Matrix2x2::FIXED_ENCODED_SIZE, 16);
}

#[test]
fn fixed_array_struct_encoded_size_matches() {
  let m = Matrix2x2::new([0.0; 4]);
  assert_eq!(m.encoded_size(), m.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Message (basic usage)
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_default_is_empty() {
  let msg = DrawCommand::default();
  assert!(msg.target.is_none());
  assert!(msg.color.is_none());
  assert!(msg.label.is_none());
  assert!(msg.thickness.is_none());
}

#[test]
fn message_round_trip_partial() {
  let mut cmd = DrawCommand::default();
  cmd.target = Some(Point::new(5.0, 10.0));
  cmd.color = Some(Color::Blue);
  // label and thickness left as None

  let bytes = cmd.to_bytes();
  let cmd2 = DrawCommand::from_bytes(&bytes).unwrap();
  assert_eq!(cmd2.color, Some(Color::Blue));
  assert!(cmd2.label.is_none());
  assert!(cmd2.thickness.is_none());
  let target = cmd2.target.unwrap();
  assert_eq!(target.x, 5.0);
  assert_eq!(target.y, 10.0);
}

#[test]
fn message_round_trip_full() {
  let mut cmd = DrawCommand::default();
  cmd.target = Some(Point::new(1.0, 2.0));
  cmd.color = Some(Color::Red);
  cmd.label = Some(Cow::Owned("test label".to_string()));
  cmd.thickness = Some(2.5);

  let bytes = cmd.to_bytes();
  let cmd2 = DrawCommand::from_bytes(&bytes).unwrap();
  assert_eq!(cmd2.target.unwrap().x, 1.0);
  assert_eq!(cmd2.color, Some(Color::Red));
  assert_eq!(cmd2.label.as_deref(), Some("test label"));
  assert_eq!(cmd2.thickness, Some(2.5));
}

#[test]
fn message_zero_copy_string_fields() {
  let mut cmd = DrawCommand::default();
  cmd.label = Some(Cow::Owned("hello".to_string()));
  let bytes = cmd.to_bytes();
  let decoded = DrawCommand::from_bytes(&bytes).unwrap();
  match &decoded.label {
    Some(Cow::Borrowed(_)) => {}
    _ => panic!("expected Cow::Borrowed"),
  }
}

#[test]
fn message_into_owned() {
  let mut cmd = DrawCommand::default();
  cmd.label = Some(Cow::Owned("owned".to_string()));
  let bytes = cmd.to_bytes();
  let decoded = DrawCommand::from_bytes(&bytes).unwrap();
  let owned: DrawCommandOwned = decoded.into_owned();
  assert_eq!(owned.label.as_deref(), Some("owned"));
}

#[test]
fn message_encoded_size_matches() {
  let mut cmd = DrawCommand::default();
  cmd.target = Some(Point::new(1.0, 2.0));
  cmd.label = Some(Cow::Owned("test".to_string()));
  assert_eq!(cmd.encoded_size(), cmd.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Message with complex field types
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_with_string_array() {
  let mut profile = UserProfile::default();
  profile.tags = Some(vec![
    Cow::Owned("rust".to_string()),
    Cow::Owned("bebop".to_string()),
  ]);
  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  let tags = p2.tags.unwrap();
  assert_eq!(tags.len(), 2);
  assert_eq!(tags[0], "rust");
  assert_eq!(tags[1], "bebop");
}

#[test]
fn message_with_string_map() {
  let mut profile = UserProfile::default();
  let mut meta = HashMap::new();
  meta.insert(
    Cow::Owned("theme".to_string()),
    Cow::Owned("dark".to_string()),
  );
  meta.insert(Cow::Owned("lang".to_string()), Cow::Owned("en".to_string()));
  profile.metadata = Some(meta);

  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  let meta2 = p2.metadata.unwrap();
  assert_eq!(meta2.len(), 2);
  assert_eq!(meta2["theme"], "dark");
  assert_eq!(meta2["lang"], "en");
}

#[test]
fn message_with_all_fields() {
  let mut profile = UserProfile::default();
  profile.display_name = Some(Cow::Owned("Alice".to_string()));
  profile.email = Some(Cow::Owned("alice@example.com".to_string()));
  profile.age = Some(30);
  profile.active = Some(true);
  profile.tags = Some(vec![Cow::Owned("admin".to_string())]);
  let mut meta = HashMap::new();
  meta.insert(Cow::Owned("key".to_string()), Cow::Owned("val".to_string()));
  profile.metadata = Some(meta);
  profile.permissions = Some(Permissions::READ | Permissions::WRITE);

  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  assert_eq!(p2.display_name.as_deref(), Some("Alice"));
  assert_eq!(p2.email.as_deref(), Some("alice@example.com"));
  assert_eq!(p2.age, Some(30));
  assert_eq!(p2.active, Some(true));
  assert_eq!(p2.tags.unwrap().len(), 1);
  assert_eq!(p2.metadata.unwrap().len(), 1);
  assert_eq!(p2.permissions, Some(Permissions::READ | Permissions::WRITE));
}

#[test]
fn message_with_all_fields_encoded_size_matches() {
  let mut profile = UserProfile::default();
  profile.display_name = Some(Cow::Owned("Alice".to_string()));
  profile.email = Some(Cow::Owned("alice@example.com".to_string()));
  profile.age = Some(30);
  profile.active = Some(true);
  profile.tags = Some(vec![
    Cow::Owned("a".to_string()),
    Cow::Owned("bb".to_string()),
  ]);
  let mut meta = HashMap::new();
  meta.insert(Cow::Owned("k".to_string()), Cow::Owned("v".to_string()));
  profile.metadata = Some(meta);
  profile.permissions = Some(Permissions::ALL);

  assert_eq!(profile.encoded_size(), profile.to_bytes().len());
}

#[test]
fn message_with_map_string_uint32() {
  let mut inv = Inventory::default();
  let mut items = HashMap::new();
  items.insert(Cow::Owned("sword".to_string()), 1u32);
  items.insert(Cow::Owned("potion".to_string()), 5u32);
  inv.items = Some(items);
  inv.label = Some(Cow::Owned("player inventory".to_string()));

  let bytes = inv.to_bytes();
  let inv2 = Inventory::from_bytes(&bytes).unwrap();
  let items2 = inv2.items.unwrap();
  assert_eq!(items2.len(), 2);
  assert_eq!(items2[&Cow::Borrowed("sword") as &Cow<str>], 1);
  assert_eq!(items2[&Cow::Borrowed("potion") as &Cow<str>], 5);
  assert_eq!(inv2.label.as_deref(), Some("player inventory"));
}

#[test]
fn message_inventory_encoded_size_matches() {
  let mut inv = Inventory::default();
  let mut items = HashMap::new();
  items.insert(Cow::Owned("a".to_string()), 10u32);
  inv.items = Some(items);
  inv.label = Some(Cow::Owned("x".to_string()));
  assert_eq!(inv.encoded_size(), inv.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Empty message
// ═══════════════════════════════════════════════════════════════

#[test]
fn empty_message_round_trip() {
  let msg = EmptyMessage::default();
  assert!(msg.unused_field.is_none());
  let bytes = msg.to_bytes();
  let msg2 = EmptyMessage::from_bytes(&bytes).unwrap();
  assert!(msg2.unused_field.is_none());
}

#[test]
fn empty_message_encoded_size_matches() {
  let msg = EmptyMessage::default();
  assert_eq!(msg.encoded_size(), msg.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Union
// ═══════════════════════════════════════════════════════════════

#[test]
fn union_fixed_branch_round_trip() {
  let shape = Shape::Point(Point::new(1.0, 2.0));
  let bytes = shape.to_bytes();
  let shape2 = Shape::from_bytes(&bytes).unwrap();
  match shape2 {
    Shape::Point(p) => {
      assert_eq!(p.x, 1.0);
      assert_eq!(p.y, 2.0);
    }
    _ => panic!("expected Shape::Point"),
  }
}

#[test]
fn union_composite_fixed_branch_round_trip() {
  let shape = Shape::Pixel(Pixel::new(Point::new(3.0, 4.0), Color::Green, 200));
  let bytes = shape.to_bytes();
  let shape2 = Shape::from_bytes(&bytes).unwrap();
  match shape2 {
    Shape::Pixel(px) => {
      assert_eq!(px.position.x, 3.0);
      assert_eq!(px.color, Color::Green);
      assert_eq!(px.alpha, 200);
    }
    _ => panic!("expected Shape::Pixel"),
  }
}

#[test]
fn union_variable_branch_round_trip() {
  let shape = Shape::Label(TextLabel::new(Point::new(0.0, 0.0), "hello"));
  let bytes = shape.to_bytes();
  let shape2 = Shape::from_bytes(&bytes).unwrap();
  match shape2 {
    Shape::Label(lbl) => {
      assert_eq!(lbl.position.x, 0.0);
      assert_eq!(lbl.text, "hello");
    }
    _ => panic!("expected Shape::Label"),
  }
}

#[test]
fn union_zero_copy_in_variable_branch() {
  let shape = Shape::Label(TextLabel::new(Point::new(0.0, 0.0), "borrowed"));
  let bytes = shape.to_bytes();
  let decoded = Shape::from_bytes(&bytes).unwrap();
  match &decoded {
    Shape::Label(lbl) => {
      assert!(matches!(lbl.text, Cow::Borrowed(_)));
    }
    _ => panic!("expected Shape::Label"),
  }
}

#[test]
fn union_into_owned() {
  let shape = Shape::Label(TextLabel::new(Point::new(1.0, 2.0), "owned_test"));
  let bytes = shape.to_bytes();
  let decoded = Shape::from_bytes(&bytes).unwrap();
  let owned: ShapeOwned = decoded.into_owned();
  match &owned {
    Shape::Label(lbl) => {
      assert!(matches!(lbl.text, Cow::Owned(_)));
      assert_eq!(lbl.text, "owned_test");
    }
    _ => panic!("expected Shape::Label"),
  }
}

#[test]
fn union_into_owned_fixed_branch() {
  // Fixed branches don't need into_owned transformation, but it should still work
  let shape = Shape::Point(Point::new(5.0, 6.0));
  let bytes = shape.to_bytes();
  let decoded = Shape::from_bytes(&bytes).unwrap();
  let owned: ShapeOwned = decoded.into_owned();
  match owned {
    Shape::Point(p) => {
      assert_eq!(p.x, 5.0);
      assert_eq!(p.y, 6.0);
    }
    _ => panic!("expected Shape::Point"),
  }
}

#[test]
fn union_encoded_size_matches() {
  let shapes: Vec<Shape> = vec![
    Shape::Point(Point::new(1.0, 2.0)),
    Shape::Pixel(Pixel::new(Point::new(0.0, 0.0), Color::Red, 128)),
    Shape::Label(TextLabel::new(Point::new(0.0, 0.0), "test")),
  ];
  for shape in shapes {
    assert_eq!(shape.encoded_size(), shape.to_bytes().len());
  }
}

#[test]
fn union_unknown_discriminator() {
  // Manually construct bytes with an unknown discriminator (99)
  let shape = Shape::Point(Point::new(1.0, 2.0));
  let mut bytes = shape.to_bytes();
  // The layout is: [u32 length][u8 discriminator][payload...]
  // discriminator is at index 4
  bytes[4] = 99;

  let decoded = Shape::from_bytes(&bytes).unwrap();
  match decoded {
    Shape::Unknown(disc, data) => {
      assert_eq!(disc, 99);
      // data should contain the remaining bytes (the Point payload)
      assert!(!data.is_empty());
    }
    _ => panic!("expected Shape::Unknown for unknown discriminator"),
  }
}

#[test]
fn union_unknown_zero_copy() {
  let shape = Shape::Point(Point::new(1.0, 2.0));
  let mut bytes = shape.to_bytes();
  bytes[4] = 99; // unknown discriminator

  let decoded = Shape::from_bytes(&bytes).unwrap();
  match &decoded {
    Shape::Unknown(_, data) => {
      assert!(matches!(data, Cow::Borrowed(_)));
    }
    _ => panic!("expected Shape::Unknown"),
  }
}

// ═══════════════════════════════════════════════════════════════
// Message with array of unions (Scene)
// ═══════════════════════════════════════════════════════════════

#[test]
fn scene_round_trip() {
  let mut scene = Scene::default();
  scene.shapes = Some(vec![
    Shape::Point(Point::new(1.0, 2.0)),
    Shape::Label(TextLabel::new(Point::new(3.0, 4.0), "label")),
  ]);
  scene.background = Some(Color::Blue);
  scene.title = Some(Cow::Owned("test scene".to_string()));

  let bytes = scene.to_bytes();
  let scene2 = Scene::from_bytes(&bytes).unwrap();
  let shapes = scene2.shapes.unwrap();
  assert_eq!(shapes.len(), 2);
  match &shapes[0] {
    Shape::Point(p) => assert_eq!(p.x, 1.0),
    _ => panic!("expected Point"),
  }
  match &shapes[1] {
    Shape::Label(lbl) => assert_eq!(lbl.text, "label"),
    _ => panic!("expected Label"),
  }
  assert_eq!(scene2.background, Some(Color::Blue));
  assert_eq!(scene2.title.as_deref(), Some("test scene"));
}

#[test]
fn scene_into_owned() {
  let mut scene = Scene::default();
  scene.shapes = Some(vec![Shape::Label(TextLabel::new(
    Point::new(0.0, 0.0),
    "x",
  ))]);
  scene.title = Some(Cow::Owned("y".to_string()));

  let bytes = scene.to_bytes();
  let decoded = Scene::from_bytes(&bytes).unwrap();
  let owned: SceneOwned = decoded.into_owned();
  assert_eq!(owned.title.as_deref(), Some("y"));
  match &owned.shapes.unwrap()[0] {
    Shape::Label(lbl) => assert!(matches!(lbl.text, Cow::Owned(_))),
    _ => panic!("expected Label"),
  }
}

#[test]
fn scene_encoded_size_matches() {
  let mut scene = Scene::default();
  scene.shapes = Some(vec![
    Shape::Point(Point::new(1.0, 2.0)),
    Shape::Pixel(Pixel::new(Point::new(0.0, 0.0), Color::Red, 255)),
  ]);
  scene.background = Some(Color::Green);
  scene.title = Some(Cow::Owned("scene title".to_string()));
  assert_eq!(scene.encoded_size(), scene.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Ergonomics: Cow covariance and owned/borrowed interop
// ═══════════════════════════════════════════════════════════════

#[test]
fn owned_type_usable_as_borrowed() {
  // Person<'static> (PersonOwned) can be used where Person<'buf> is expected
  // because Cow<'static, str> is covariant over the lifetime
  let owned: PersonOwned = Person::new("test", 1);

  fn accepts_person(p: &Person) {
    assert_eq!(p.name, "test");
  }

  accepts_person(&owned);
}

#[test]
fn borrowed_can_outlive_buffer_via_into_owned() {
  let owned: PersonOwned = {
    let bytes = Person::new("temp", 42).to_bytes();
    let decoded = Person::from_bytes(&bytes).unwrap();
    decoded.into_owned()
    // bytes dropped here
  };
  // owned survives past the buffer's lifetime
  assert_eq!(owned.name, "temp");
  assert_eq!(owned.age, 42);
}

#[test]
fn constructing_cow_fields_directly() {
  // Users can construct Cow fields manually for borrowed data
  let name = "direct_borrow";
  let p = Person {
    name: Cow::Borrowed(name),
    age: 100,
  };
  assert_eq!(p.name, "direct_borrow");

  // And it round-trips
  let bytes = p.to_bytes();
  let p2 = Person::from_bytes(&bytes).unwrap();
  assert_eq!(p2.name, "direct_borrow");
}

// ═══════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════

#[test]
fn message_empty_string_array() {
  let mut profile = UserProfile::default();
  profile.tags = Some(vec![]);
  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  assert_eq!(p2.tags.unwrap().len(), 0);
}

#[test]
fn message_empty_map() {
  let mut profile = UserProfile::default();
  profile.metadata = Some(HashMap::new());
  let bytes = profile.to_bytes();
  let p2 = UserProfile::from_bytes(&bytes).unwrap();
  assert_eq!(p2.metadata.unwrap().len(), 0);
}

#[test]
fn unicode_string_round_trip() {
  let p = Person::new("Hello 世界 🌍", 1);
  let bytes = p.to_bytes();
  let p2 = Person::from_bytes(&bytes).unwrap();
  assert_eq!(p2.name, "Hello 世界 🌍");
}

#[test]
fn large_byte_array() {
  let data = vec![0xABu8; 10_000];
  let bp = BinaryPayload::new(1, data.clone());
  let bytes = bp.to_bytes();
  let bp2 = BinaryPayload::from_bytes(&bytes).unwrap();
  assert_eq!(bp2.data, &data[..]);
  assert_eq!(bp.encoded_size(), bytes.len());
}

#[test]
fn multiple_decode_from_same_buffer() {
  // Decode the same buffer multiple times — all should borrow from it
  let p = Person::new("shared", 1);
  let bytes = p.to_bytes();

  let d1 = Person::from_bytes(&bytes).unwrap();
  let d2 = Person::from_bytes(&bytes).unwrap();
  assert_eq!(d1.name, d2.name);
  assert!(matches!(d1.name, Cow::Borrowed(_)));
  assert!(matches!(d2.name, Cow::Borrowed(_)));
}

// ═══════════════════════════════════════════════════════════════
// Additional coverage
// ═══════════════════════════════════════════════════════════════

#[test]
fn forward_reference_struct_round_trip() {
  let a = ForwardRefA::new(ForwardRefB::new("defined later"));
  let bytes = a.to_bytes();
  let decoded = ForwardRefA::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.b.value, "defined later");
  assert_eq!(a.encoded_size(), bytes.len());
}

#[test]
#[allow(deprecated)]
fn deprecated_message_fields_round_trip() {
  let mut msg = DeprecatedFieldsMessage::default();
  msg.current_name = Some(Cow::Owned("current".to_string()));
  msg.legacy_name = Some(Cow::Owned("legacy".to_string()));
  msg.legacy_enabled = Some(true);

  let bytes = msg.to_bytes();
  let decoded = DeprecatedFieldsMessage::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.current_name.as_deref(), Some("current"));
  assert_eq!(decoded.legacy_name.as_deref(), Some("legacy"));
  assert_eq!(decoded.legacy_enabled, Some(true));
  assert_eq!(msg.encoded_size(), bytes.len());
}

#[test]
fn integer_key_maps_round_trip() {
  let mut msg = IntegerKeyMaps::default();
  let mut labels = HashMap::new();
  labels.insert(7u32, Cow::Owned("seven".to_string()));
  labels.insert(42u32, Cow::Owned("forty-two".to_string()));
  let mut flags = HashMap::new();
  flags.insert(-1i64, false);
  flags.insert(1_000_000_000_000i64, true);
  msg.labels_by_id = Some(labels);
  msg.flags_by_id = Some(flags);

  let bytes = msg.to_bytes();
  let decoded = IntegerKeyMaps::from_bytes(&bytes).unwrap();
  let labels2 = decoded.labels_by_id.unwrap();
  let flags2 = decoded.flags_by_id.unwrap();
  assert_eq!(labels2[&7], "seven");
  assert_eq!(labels2[&42], "forty-two");
  assert!(!flags2[&-1]);
  assert!(flags2[&1_000_000_000_000]);
  assert_eq!(msg.encoded_size(), bytes.len());
}

#[test]
fn integer_key_maps_empty_round_trip() {
  let mut msg = IntegerKeyMaps::default();
  msg.labels_by_id = Some(HashMap::new());
  msg.flags_by_id = Some(HashMap::new());

  let bytes = msg.to_bytes();
  let decoded = IntegerKeyMaps::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.labels_by_id.unwrap().len(), 0);
  assert_eq!(decoded.flags_by_id.unwrap().len(), 0);
}

#[test]
fn deep_nested_collections_round_trip() {
  let mut branch_a = HashMap::new();
  branch_a.insert(Cow::Owned("left".to_string()), NestedLeaf::new("alpha"));
  let mut branch_b = HashMap::new();
  branch_b.insert(Cow::Owned("right".to_string()), NestedLeaf::new("beta"));

  let mut nested = HashMap::new();
  nested.insert(Cow::Owned("bucket".to_string()), vec![branch_a, branch_b]);

  let mut msg = DeepNestedCollections::default();
  msg.nested = Some(nested);

  let bytes = msg.to_bytes();
  let decoded = DeepNestedCollections::from_bytes(&bytes).unwrap();
  let nested2 = decoded.nested.unwrap();
  let bucket = &nested2[&Cow::Borrowed("bucket") as &Cow<str>];
  assert_eq!(bucket.len(), 2);
  assert_eq!(
    bucket[0][&Cow::Borrowed("left") as &Cow<str>].label,
    "alpha"
  );
  assert_eq!(
    bucket[1][&Cow::Borrowed("right") as &Cow<str>].label,
    "beta"
  );
  assert_eq!(msg.encoded_size(), bytes.len());
}

#[test]
fn deep_nested_collections_empty_round_trip() {
  let mut msg = DeepNestedCollections::default();
  let mut nested = HashMap::new();
  nested.insert(Cow::Owned("bucket".to_string()), Vec::new());
  msg.nested = Some(nested);

  let bytes = msg.to_bytes();
  let decoded = DeepNestedCollections::from_bytes(&bytes).unwrap();
  let nested2 = decoded.nested.unwrap();
  assert!(nested2[&Cow::Borrowed("bucket") as &Cow<str>].is_empty());
}

// ═══════════════════════════════════════════════════════════════
// Temporal types
// ═══════════════════════════════════════════════════════════════

#[test]
fn timestamped_event_round_trip() {
  let ts = BebopTimestamp {
    seconds: 1_700_000_000,
    nanos: 123_456_789,
  };
  let evt = TimestampedEvent::new(ts, "deploy");
  let bytes = evt.to_bytes();
  let evt2 = TimestampedEvent::from_bytes(&bytes).unwrap();
  assert_eq!(evt2.when, ts);
  assert_eq!(evt2.what, "deploy");
}

#[test]
fn timestamped_event_encoded_size_matches() {
  let ts = BebopTimestamp {
    seconds: 0,
    nanos: 0,
  };
  let evt = TimestampedEvent::new(ts, "test");
  assert_eq!(evt.encoded_size(), evt.to_bytes().len());
}

#[test]
fn schedule_entry_round_trip_full() {
  let mut entry = ScheduleEntry::default();
  entry.start = Some(BebopTimestamp {
    seconds: 1_700_000_000,
    nanos: 0,
  });
  entry.duration = Some(BebopDuration {
    seconds: 3600,
    nanos: 0,
  });
  entry.label = Some(Cow::Owned("meeting".to_string()));

  let bytes = entry.to_bytes();
  let entry2 = ScheduleEntry::from_bytes(&bytes).unwrap();
  assert_eq!(
    entry2.start,
    Some(BebopTimestamp {
      seconds: 1_700_000_000,
      nanos: 0
    })
  );
  assert_eq!(
    entry2.duration,
    Some(BebopDuration {
      seconds: 3600,
      nanos: 0
    })
  );
  assert_eq!(entry2.label.as_deref(), Some("meeting"));
}

#[test]
fn schedule_entry_round_trip_partial() {
  let mut entry = ScheduleEntry::default();
  entry.start = Some(BebopTimestamp {
    seconds: 100,
    nanos: 500,
  });
  // duration and label left as None

  let bytes = entry.to_bytes();
  let entry2 = ScheduleEntry::from_bytes(&bytes).unwrap();
  assert_eq!(
    entry2.start,
    Some(BebopTimestamp {
      seconds: 100,
      nanos: 500
    })
  );
  assert!(entry2.duration.is_none());
  assert!(entry2.label.is_none());
}

#[test]
fn schedule_entry_encoded_size_matches() {
  let mut entry = ScheduleEntry::default();
  entry.start = Some(BebopTimestamp {
    seconds: 42,
    nanos: 1,
  });
  entry.duration = Some(BebopDuration {
    seconds: 10,
    nanos: 999,
  });
  entry.label = Some(Cow::Owned("x".to_string()));
  assert_eq!(entry.encoded_size(), entry.to_bytes().len());
}

// ═══════════════════════════════════════════════════════════════
// Serde
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "serde")]
#[test]
fn serde_struct_round_trip_json() {
  let person = Person::new("Alice", 30);
  let json = serde_json::to_string(&person).unwrap();
  let decoded: PersonOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.name, "Alice");
  assert_eq!(decoded.age, 30);
}

#[cfg(feature = "serde")]
#[test]
fn serde_byte_array_struct_round_trip_json() {
  let payload = BinaryPayload::new(7, vec![1, 2, 3, 4]);
  let json = serde_json::to_string(&payload).unwrap();
  let decoded: BinaryPayload = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.tag, 7);
  assert_eq!(decoded.data.as_ref(), &[1, 2, 3, 4]);
}

#[cfg(feature = "serde")]
#[test]
fn serde_message_round_trip_json() {
  let mut profile = UserProfile::default();
  profile.display_name = Some(Cow::Owned("alice".to_string()));
  profile.email = Some(Cow::Owned("alice@example.com".to_string()));
  profile.age = Some(42);
  profile.active = Some(true);
  profile.tags = Some(vec![Cow::Owned("admin".to_string())]);
  let mut meta = HashMap::new();
  meta.insert(
    Cow::Owned("language".to_string()),
    Cow::Owned("rust".to_string()),
  );
  profile.metadata = Some(meta);
  profile.permissions = Some(Permissions::READ | Permissions::WRITE);

  let json = serde_json::to_string(&profile).unwrap();
  let decoded: UserProfileOwned = serde_json::from_str(&json).unwrap();
  assert_eq!(decoded.display_name.as_deref(), Some("alice"));
  assert_eq!(decoded.email.as_deref(), Some("alice@example.com"));
  assert_eq!(decoded.age, Some(42));
  assert_eq!(decoded.active, Some(true));
  assert_eq!(
    decoded.permissions,
    Some(Permissions::READ | Permissions::WRITE)
  );
}

#[cfg(feature = "serde")]
#[test]
fn serde_enum_and_flags_round_trip_json() {
  let enum_json = serde_json::to_string(&Color::Blue).unwrap();
  let enum_decoded: Color = serde_json::from_str(&enum_json).unwrap();
  assert_eq!(enum_decoded, Color::Blue);

  let flags_json = serde_json::to_string(&(Permissions::READ | Permissions::WRITE)).unwrap();
  let flags_decoded: Permissions = serde_json::from_str(&flags_json).unwrap();
  assert_eq!(flags_decoded, Permissions::READ | Permissions::WRITE);
}

#[cfg(feature = "serde")]
#[test]
fn serde_union_round_trip_json() {
  let shape = Shape::Label(TextLabel::new(Point::new(1.0, 2.0), "json label"));
  let json = serde_json::to_string(&shape).unwrap();
  let decoded: ShapeOwned = serde_json::from_str(&json).unwrap();
  match decoded {
    Shape::Label(label) => {
      assert_eq!(label.position.x, 1.0);
      assert_eq!(label.position.y, 2.0);
      assert_eq!(label.text, "json label");
    }
    _ => panic!("expected Shape::Label"),
  }
}

// ═══════════════════════════════════════════════════════════════
// Forward-compatible enum (@forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn fc_enum_known_round_trip() {
  let p = Priority::High;
  let bytes = p.to_bytes();
  let decoded = Priority::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, Priority::High);
  assert!(decoded.is_known());
  assert_eq!(decoded.discriminator(), 3);
}

#[test]
fn fc_enum_unknown_value() {
  // Manually write an unknown discriminator (42) for a uint8 enum
  let bytes = [42u8];
  let decoded = Priority::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, Priority::Unknown(42));
  assert!(!decoded.is_known());
  assert_eq!(decoded.discriminator(), 42);
}

#[test]
fn fc_enum_unknown_round_trip() {
  let p = Priority::Unknown(99);
  let bytes = p.to_bytes();
  let decoded = Priority::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, Priority::Unknown(99));
}

#[test]
fn strict_enum_rejects_unknown() {
  // Color is a strict enum (no @forward_compatible).
  // An unknown discriminator (42) should fail.
  let bytes = [42u8];
  let err = Color::from_bytes(&bytes).unwrap_err();
  assert!(matches!(err, DecodeError::InvalidEnum { .. }));
}

// ═══════════════════════════════════════════════════════════════
// Strict union (no @forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn strict_union_known_round_trip() {
  let s = StrictShape::Point(Point::new(3.0, 4.0));
  let bytes = s.to_bytes();
  let decoded = StrictShape::from_bytes(&bytes).unwrap();
  match decoded {
    StrictShape::Point(p) => {
      assert_eq!(p.x, 3.0);
      assert_eq!(p.y, 4.0);
    }
    _ => panic!("expected StrictShape::Point"),
  }
}

#[test]
fn strict_union_rejects_unknown_discriminator() {
  let s = StrictShape::Point(Point::new(1.0, 2.0));
  let mut bytes = s.to_bytes();
  // Union layout: [u32 length][u8 discriminator][payload...]
  bytes[4] = 99;
  let err = StrictShape::from_bytes(&bytes).unwrap_err();
  assert!(matches!(err, DecodeError::InvalidUnion { .. }));
}

// ═══════════════════════════════════════════════════════════════
// Strict message (no @forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn strict_message_known_fields_round_trip() {
  let mut msg = StrictConfig::default();
  msg.name = Some(Cow::Borrowed("test"));
  msg.value = Some(42);
  let bytes = msg.to_bytes();
  let decoded = StrictConfig::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.name.as_deref(), Some("test"));
  assert_eq!(decoded.value, Some(42));
}

#[test]
fn strict_message_rejects_unknown_field() {
  // Build a valid StrictConfig, then inject an extra field tag.
  // StrictConfig has fields 1 (string) and 2 (u32). Tag 99 is unknown.
  // Message wire format: [u32 body_len] [tag, value]* [0x00 terminator]
  let mut bytes = Vec::new();
  // We'll write a message with just an unknown tag 99 followed by terminator.
  // The body will be: [tag=99] [... we need skip-able data] [tag=0]
  // For a strict message, the decoder returns error on unknown tag,
  // so we just need the unknown tag to be present.
  // body: tag 99, then tag 0 (terminator)
  let body: Vec<u8> = vec![99, 0];
  let body_len = body.len() as u32;
  bytes.extend_from_slice(&body_len.to_le_bytes());
  bytes.extend_from_slice(&body);

  let err = StrictConfig::from_bytes(&bytes).unwrap_err();
  assert!(matches!(err, DecodeError::InvalidField { .. }));
}

// ═══════════════════════════════════════════════════════════════
// Forward-compatible message (@forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn fc_message_known_fields_round_trip() {
  let mut msg = FlexConfig::default();
  msg.name = Some(Cow::Borrowed("flex"));
  msg.value = Some(7);
  let bytes = msg.to_bytes();
  let decoded = FlexConfig::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.name.as_deref(), Some("flex"));
  assert_eq!(decoded.value, Some(7));
}

#[test]
fn fc_message_skips_unknown_field() {
  // Encode a FlexConfig with known fields, then inject extra trailing data
  // before the terminator. The fc message should skip it gracefully.
  let mut msg = FlexConfig::default();
  msg.name = Some(Cow::Borrowed("ok"));
  let bytes = msg.to_bytes();
  // If we decode the valid bytes, it should work fine.
  let decoded = FlexConfig::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.name.as_deref(), Some("ok"));

  // Now build a message with an unknown tag 99 + some payload.
  // The fc decoder should skip to the end without error.
  // body: [tag=99] then skip to end, [tag=0] terminator
  let body: Vec<u8> = vec![99, 0];
  let body_len = body.len() as u32;
  let mut bytes = Vec::new();
  bytes.extend_from_slice(&body_len.to_le_bytes());
  bytes.extend_from_slice(&body);

  let decoded = FlexConfig::from_bytes(&bytes).unwrap();
  // All fields should be None since we only sent unknown tag
  assert_eq!(decoded.name, None);
  assert_eq!(decoded.value, None);
}

// ═══════════════════════════════════════════════════════════════
// Strict flags (no @forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn strict_flags_valid_round_trip() {
  let p = Permissions::READ | Permissions::WRITE;
  let bytes = p.to_bytes();
  let decoded = Permissions::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, p);
}

#[test]
fn strict_flags_rejects_unknown_bits() {
  // Permissions has ALL_BITS = 7 (READ=1, WRITE=2, EXECUTE=4).
  // Bit 128 is not defined, so strict decode should reject.
  let bytes = [128u8];
  let err = Permissions::from_bytes(&bytes).unwrap_err();
  assert!(matches!(err, DecodeError::InvalidFlags { .. }));
}

// ═══════════════════════════════════════════════════════════════
// Forward-compatible flags (@forward_compatible)
// ═══════════════════════════════════════════════════════════════

#[test]
fn fc_flags_valid_round_trip() {
  let p = FlexPermissions::READ | FlexPermissions::WRITE;
  let bytes = p.to_bytes();
  let decoded = FlexPermissions::from_bytes(&bytes).unwrap();
  assert_eq!(decoded, p);
}

#[test]
fn fc_flags_accepts_unknown_bits() {
  // FlexPermissions has ALL_BITS = 3 (READ=1, WRITE=2).
  // With @forward_compatible, bit 128 should be accepted.
  let bytes = [128u8];
  let decoded = FlexPermissions::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.bits(), 128);
}

#[test]
fn fc_flags_unknown_bits_round_trip() {
  // Unknown bits should survive encode→decode round-trip.
  let val = FlexPermissions::READ | FlexPermissions(128);
  let bytes = val.to_bytes();
  let decoded = FlexPermissions::from_bytes(&bytes).unwrap();
  assert_eq!(decoded.bits(), 129); // 1 | 128
}
