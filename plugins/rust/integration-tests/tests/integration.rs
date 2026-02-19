use std::borrow::Cow;
use std::collections::HashMap;

use bebop_runtime::{BebopDecode, BebopEncode, BebopFlags};

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
    assert_eq!(EXAMPLE_CONST_FALSE, false);
    assert_eq!(EXAMPLE_CONST_TRUE, true);
    assert_eq!(HTTP_STATUS_OK, 200);
    assert_eq!(FEATURE_FLAG_ENABLED, true);
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
        [
            0xE2, 0x15, 0xA9, 0x46, 0xB2, 0x6F, 0x45, 0x67, 0xA2, 0x76, 0x13, 0x13, 0x6F, 0x0A,
            0x17, 0x08,
        ]
    );
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
    let p = Point::new(3.14, 2.71);
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
    meta.insert(
        Cow::Owned("lang".to_string()),
        Cow::Owned("en".to_string()),
    );
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
    meta.insert(
        Cow::Owned("key".to_string()),
        Cow::Owned("val".to_string()),
    );
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
    meta.insert(
        Cow::Owned("k".to_string()),
        Cow::Owned("v".to_string()),
    );
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
    scene.shapes = Some(vec![
        Shape::Label(TextLabel::new(Point::new(0.0, 0.0), "x")),
    ]);
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
