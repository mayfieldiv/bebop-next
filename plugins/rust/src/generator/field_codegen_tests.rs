use super::*;
use crate::generated::{
  DefinitionDescriptor, DefinitionKind, FieldDescriptor, SchemaDescriptor, StructDef,
};
use std::borrow::Cow;

fn td_scalar(kind: TypeKind) -> TypeDescriptor<'static> {
  TypeDescriptor {
    kind: Some(kind),
    ..Default::default()
  }
}

fn td_array(elem: TypeDescriptor<'static>) -> TypeDescriptor<'static> {
  TypeDescriptor {
    kind: Some(TypeKind::Array),
    array_element: Some(Box::new(elem)),
    ..Default::default()
  }
}

fn td_fixed_array(elem: TypeDescriptor<'static>, size: u32) -> TypeDescriptor<'static> {
  TypeDescriptor {
    kind: Some(TypeKind::FixedArray),
    fixed_array_element: Some(Box::new(elem)),
    fixed_array_size: Some(size),
    ..Default::default()
  }
}

fn td_map(key: TypeDescriptor<'static>, val: TypeDescriptor<'static>) -> TypeDescriptor<'static> {
  TypeDescriptor {
    kind: Some(TypeKind::Map),
    map_key: Some(Box::new(key)),
    map_value: Some(Box::new(val)),
    ..Default::default()
  }
}

fn td_defined(fqn: &'static str) -> TypeDescriptor<'static> {
  TypeDescriptor {
    kind: Some(TypeKind::Defined),
    defined_fqn: Some(Cow::Borrowed(fqn)),
    ..Default::default()
  }
}

fn empty_analysis() -> SchemaAnalysis {
  SchemaAnalysis::build(&[])
}

fn analysis_with_lifetime(fqn: &str) -> SchemaAnalysis {
  let def = DefinitionDescriptor {
    kind: Some(DefinitionKind::Struct),
    name: Some(Cow::Owned(
      fqn.rsplit('.').next().unwrap_or(fqn).to_string(),
    )),
    fqn: Some(Cow::Owned(fqn.to_string())),
    struct_def: Some(StructDef {
      fields: Some(vec![FieldDescriptor {
        name: Some(Cow::Borrowed("name")),
        r#type: Some(td_scalar(TypeKind::String)),
        index: Some(0),
        ..Default::default()
      }]),
      ..Default::default()
    }),
    ..Default::default()
  };
  let schema = SchemaDescriptor {
    definitions: Some(vec![def]),
    ..Default::default()
  };
  SchemaAnalysis::build(std::slice::from_ref(&schema))
}

// ── FixedScalarInfo table tests ─────────────────────────────────

#[test]
fn fixed_scalar_table_has_18_entries() {
  assert_eq!(FIXED_SCALAR_TABLE.len(), 18);
}

#[test]
fn fixed_scalar_info_returns_none_for_string() {
  assert!(fixed_scalar_info(TypeKind::String).is_none());
}

#[test]
fn fixed_scalar_info_returns_none_for_compound_types() {
  assert!(fixed_scalar_info(TypeKind::Array).is_none());
  assert!(fixed_scalar_info(TypeKind::Map).is_none());
  assert!(fixed_scalar_info(TypeKind::Defined).is_none());
  assert!(fixed_scalar_info(TypeKind::FixedArray).is_none());
  assert!(fixed_scalar_info(TypeKind::Unknown).is_none());
}

#[test]
fn fixed_scalar_info_returns_correct_entry_for_int32() {
  let info = fixed_scalar_info(TypeKind::Int32).unwrap();
  assert_eq!(info.rust_type, "i32");
  assert_eq!(info.read_method, "read_i32");
  assert_eq!(info.write_method, "write_i32");
  assert_eq!(info.wire_size, 4);
  assert_eq!(info.wire_size_expr, "mem::size_of::<i32>()");
  assert!(info.is_bulk_eligible);
  assert!(info.is_enum_base);
}

#[test]
fn fixed_scalar_info_bool_is_not_bulk_eligible() {
  let info = fixed_scalar_info(TypeKind::Bool).unwrap();
  assert!(!info.is_bulk_eligible);
  assert!(!info.is_enum_base);
}

#[test]
fn fixed_scalar_info_byte_is_not_bulk_eligible() {
  let info = fixed_scalar_info(TypeKind::Byte).unwrap();
  assert!(!info.is_bulk_eligible);
  assert!(info.is_enum_base);
}

#[test]
fn fixed_scalar_info_uuid() {
  let info = fixed_scalar_info(TypeKind::Uuid).unwrap();
  assert_eq!(info.rust_type, "bebop::Uuid");
  assert_eq!(info.wire_size, 16);
  assert!(!info.is_bulk_eligible);
  assert!(!info.is_enum_base);
}

#[test]
fn fixed_scalar_info_timestamp() {
  let info = fixed_scalar_info(TypeKind::Timestamp).unwrap();
  assert_eq!(info.rust_type, "bebop::BebopTimestamp");
  assert_eq!(info.wire_size, 12);
  assert!(!info.is_bulk_eligible);
}

#[test]
fn all_fixed_scalars_have_consistent_sizes() {
  for info in FIXED_SCALAR_TABLE {
    assert!(info.wire_size > 0);
    assert!(!info.rust_type.is_empty());
    assert!(!info.read_method.is_empty());
    assert!(!info.write_method.is_empty());
    assert!(!info.wire_size_expr.is_empty());
  }
}

#[test]
fn enum_base_types_are_integer_only() {
  for info in FIXED_SCALAR_TABLE {
    if info.is_enum_base {
      assert!(
        matches!(
          info.kind,
          TypeKind::Byte
            | TypeKind::Int8
            | TypeKind::Int16
            | TypeKind::Uint16
            | TypeKind::Int32
            | TypeKind::Uint32
            | TypeKind::Int64
            | TypeKind::Uint64
        ),
        "unexpected enum base: {:?}",
        info.kind
      );
    }
  }
}

// ── FieldCodegen cow_type tests ─────────────────────────────────

#[test]
fn cow_type_fixed_scalars() {
  let analysis = empty_analysis();
  for info in FIXED_SCALAR_TABLE {
    let td = td_scalar(info.kind);
    let fc = FieldCodegen::new(&td, &analysis).unwrap();
    assert_eq!(fc.cow_type(), info.rust_type, "kind: {:?}", info.kind);
  }
}

#[test]
fn cow_type_string() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "borrow::Cow<'buf, str>");
}

#[test]
fn cow_type_byte_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Byte));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "bebop::BebopBytes<'buf>");
}

#[test]
fn cow_type_bulk_scalar_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Int32));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "borrow::Cow<'buf, [i32]>");
}

#[test]
fn cow_type_string_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::String));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "vec::Vec<borrow::Cow<'buf, str>>");
}

#[test]
fn cow_type_map() {
  let analysis = empty_analysis();
  let td = td_map(td_scalar(TypeKind::String), td_scalar(TypeKind::Uint32));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "bebop::HashMap<borrow::Cow<'buf, str>, u32>");
}

#[test]
fn cow_type_defined_with_lifetime() {
  let analysis = analysis_with_lifetime("test.Foo");
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "Foo<'buf>");
}

#[test]
fn cow_type_defined_without_lifetime() {
  let analysis = empty_analysis();
  let td = td_defined("test.Bar");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "Bar");
}

#[test]
fn cow_type_fixed_array() {
  let analysis = empty_analysis();
  let td = td_fixed_array(td_scalar(TypeKind::Float32), 4);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.cow_type(), "[f32; 4]");
}

// ── FieldCodegen write_expr tests ───────────────────────────────

#[test]
fn write_expr_int32() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.x", "writer").unwrap(),
    "writer.write_i32(self.x)"
  );
}

#[test]
fn write_expr_string() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.name", "writer").unwrap(),
    "writer.write_string(&self.name)"
  );
}

#[test]
fn write_expr_byte_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Byte));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.data", "writer").unwrap(),
    "writer.write_byte_array(&self.data)"
  );
}

#[test]
fn write_expr_bulk_scalar_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Float32));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.values", "writer").unwrap(),
    "writer.write_scalar_array::<f32>(&self.values)"
  );
}

#[test]
fn write_expr_defined() {
  let analysis = empty_analysis();
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.foo", "writer").unwrap(),
    "self.foo.encode(writer)"
  );
}

// ── FieldCodegen read_expr tests ────────────────────────────────

#[test]
fn read_expr_int32() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.read_expr("reader", "MyStruct", "x").unwrap(),
    "reader.read_i32().for_field(\"MyStruct\", \"x\")?"
  );
}

#[test]
fn read_expr_string() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.read_expr("reader", "MyStruct", "name").unwrap(),
    "borrow::Cow::Borrowed(reader.read_str().for_field(\"MyStruct\", \"name\")?)"
  );
}

#[test]
fn read_expr_byte_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Byte));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.read_expr("reader", "MyStruct", "data").unwrap(),
    "bebop::BebopBytes::borrowed(reader.read_byte_slice().for_field(\"MyStruct\", \"data\")?)"
  );
}

#[test]
fn read_expr_defined() {
  let analysis = empty_analysis();
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.read_expr("reader", "MyStruct", "foo").unwrap(),
    "Foo::decode(reader).for_field(\"MyStruct\", \"foo\")?"
  );
}

// ── FieldCodegen size_expr tests ────────────────────────────────

#[test]
fn size_expr_int32() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.size_expr("self.x").unwrap(), "mem::size_of::<i32>()");
}

#[test]
fn size_expr_string() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.size_expr("self.name").unwrap(),
    "bebop::wire_size::string_size(self.name.len())"
  );
}

#[test]
fn size_expr_defined() {
  let analysis = empty_analysis();
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.size_expr("self.foo").unwrap(), "self.foo.encoded_size()");
}

// ── FieldCodegen owned_expr tests ───────────────────────────────

#[test]
fn into_owned_string() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.owned_expr("self.name").unwrap(),
    "borrow::Cow::Owned(self.name.into_owned())"
  );
}

#[test]
fn into_owned_scalar_passthrough() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.owned_expr("self.x").unwrap(), "self.x");
}

#[test]
fn into_owned_defined_with_lifetime() {
  let analysis = analysis_with_lifetime("test.Foo");
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.owned_expr("self.foo").unwrap(), "self.foo.into_owned()");
}

// ── FieldCodegen constructor_param tests ────────────────────────

#[test]
fn constructor_param_scalar() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  let cp = fc.constructor_param("x", false).unwrap();
  assert_eq!(cp.param_type, "i32");
  assert_eq!(cp.init_expr, "x");
}

#[test]
fn constructor_param_string_with_lifetime() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  let cp = fc.constructor_param("name", true).unwrap();
  assert_eq!(cp.param_type, "impl convert::Into<borrow::Cow<'buf, str>>");
  assert_eq!(cp.init_expr, "name.into()");
}

#[test]
fn constructor_param_string_without_lifetime() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  let cp = fc.constructor_param("name", false).unwrap();
  assert_eq!(cp.param_type, "string::String");
  assert_eq!(cp.init_expr, "borrow::Cow::Owned(name)");
}

#[test]
fn constructor_param_string_array() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::String));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  let cp = fc.constructor_param("tokens", true).unwrap();
  assert!(cp.param_type.contains("IntoIterator"));
  assert!(cp.init_expr.contains(".into_iter()"));
}

#[test]
fn constructor_param_map() {
  let analysis = empty_analysis();
  let td = td_map(td_scalar(TypeKind::String), td_scalar(TypeKind::Uint32));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  let cp = fc.constructor_param("entries", true).unwrap();
  assert!(cp.param_type.contains("IntoIterator"));
}

// ── FieldCodegen wire_size_expr tests ───────────────────────

#[test]
fn wire_size_expr_scalar() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::Int32);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.wire_size_expr(), Some("mem::size_of::<i32>()"));
}

#[test]
fn wire_size_expr_string_is_none() {
  let analysis = empty_analysis();
  let td = td_scalar(TypeKind::String);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert!(fc.wire_size_expr().is_none());
}

#[test]
fn wire_size_expr_fixed_array() {
  let analysis = empty_analysis();
  let td = td_fixed_array(td_scalar(TypeKind::Float32), 3);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.wire_size_expr(), Some("mem::size_of::<f32>() * 3"));
}

#[test]
fn wire_size_expr_defined() {
  let analysis = empty_analysis();
  let td = td_defined("test.Foo");
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(fc.wire_size_expr(), Some("Foo::FIXED_ENCODED_SIZE"));
}

#[test]
fn wire_size_expr_variable_array_is_none() {
  let analysis = empty_analysis();
  let td = td_array(td_scalar(TypeKind::Int32));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert!(fc.wire_size_expr().is_none());
}

// ── Compound type tests ─────────────────────────────────────────

#[test]
fn nested_vec_map_cow() {
  let analysis = empty_analysis();
  let td = td_array(td_map(
    td_scalar(TypeKind::String),
    td_scalar(TypeKind::Uint32),
  ));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.cow_type(),
    "vec::Vec<bebop::HashMap<borrow::Cow<'buf, str>, u32>>"
  );
}

#[test]
fn write_expr_array_of_defined() {
  let analysis = empty_analysis();
  let td = td_array(td_defined("test.Foo"));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.write_expr("self.foos", "writer").unwrap(),
    "writer.write_array(&self.foos, |_w, _el| _el.encode(_w))"
  );
}

#[test]
fn read_expr_array_of_defined() {
  let analysis = empty_analysis();
  let td = td_array(td_defined("test.Foo"));
  let fc = FieldCodegen::new(&td, &analysis).unwrap();
  assert_eq!(
    fc.read_expr("reader", "MyStruct", "foos").unwrap(),
    "reader.read_array(|_r| Foo::decode(_r)).for_field(\"MyStruct\", \"foos\")?"
  );
}

// ── FixedArray non-FixedScalar tests (Uuid, Timestamp, Duration) ────

#[test]
fn fixed_array_uuid_uses_element_loop_not_read_fixed_array() {
  let analysis = empty_analysis();
  let td = td_fixed_array(td_scalar(TypeKind::Uuid), 4);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();

  let read = fc.read_expr("reader", "MyStruct", "ids").unwrap();
  // Must NOT contain read_fixed_array (Uuid doesn't implement FixedScalar)
  assert!(
    !read.contains("read_fixed_array"),
    "Uuid fixed array should use element loop, got: {}",
    read
  );
  assert!(read.contains("Default::default()"), "should use manual loop");

  let write = fc.write_expr("self.ids", "writer").unwrap();
  assert!(
    !write.contains("write_fixed_array"),
    "Uuid fixed array should use element loop, got: {}",
    write
  );
  assert!(write.contains("for _el in"), "should iterate elements");
}

#[test]
fn fixed_array_timestamp_uses_element_loop() {
  let analysis = empty_analysis();
  let td = td_fixed_array(td_scalar(TypeKind::Timestamp), 2);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();

  let read = fc.read_expr("reader", "MyStruct", "times").unwrap();
  assert!(!read.contains("read_fixed_array"), "Timestamp: {}", read);

  let write = fc.write_expr("self.times", "writer").unwrap();
  assert!(!write.contains("write_fixed_array"), "Timestamp: {}", write);
}

#[test]
fn fixed_array_i32_uses_read_fixed_array() {
  let analysis = empty_analysis();
  let td = td_fixed_array(td_scalar(TypeKind::Int32), 3);
  let fc = FieldCodegen::new(&td, &analysis).unwrap();

  let read = fc.read_expr("reader", "MyStruct", "vals").unwrap();
  assert!(
    read.contains("read_fixed_array::<i32, 3>"),
    "i32 fixed array should use read_fixed_array, got: {}",
    read
  );

  let write = fc.write_expr("self.vals", "writer").unwrap();
  assert!(
    write.contains("write_fixed_array::<i32, 3>"),
    "i32 fixed array should use write_fixed_array, got: {}",
    write
  );
}

#[test]
fn is_fixed_array_scalar_matches_runtime_fixed_scalar_trait() {
  // The 15 types with FixedScalar impls in the runtime
  let fixed_scalar_kinds = [
    TypeKind::Bool,
    TypeKind::Byte,
    TypeKind::Int8,
    TypeKind::Int16,
    TypeKind::Uint16,
    TypeKind::Int32,
    TypeKind::Uint32,
    TypeKind::Int64,
    TypeKind::Uint64,
    TypeKind::Int128,
    TypeKind::Uint128,
    TypeKind::Float16,
    TypeKind::Bfloat16,
    TypeKind::Float32,
    TypeKind::Float64,
  ];
  for kind in fixed_scalar_kinds {
    let info = fixed_scalar_info(kind).unwrap();
    assert!(
      info.is_fixed_array_scalar,
      "{:?} should have is_fixed_array_scalar = true",
      kind
    );
  }

  // The 3 types without FixedScalar impls
  let non_fixed_scalar_kinds = [TypeKind::Uuid, TypeKind::Timestamp, TypeKind::Duration];
  for kind in non_fixed_scalar_kinds {
    let info = fixed_scalar_info(kind).unwrap();
    assert!(
      !info.is_fixed_array_scalar,
      "{:?} should have is_fixed_array_scalar = false",
      kind
    );
  }
}

#[test]
fn is_cow_field_variants() {
  assert!(is_cow_field(&td_scalar(TypeKind::String)));
  assert!(is_cow_field(&td_array(td_scalar(TypeKind::Byte))));
  assert!(is_cow_field(&td_array(td_scalar(TypeKind::Int32))));
  assert!(!is_cow_field(&td_scalar(TypeKind::Int32)));
  assert!(!is_cow_field(&td_array(td_scalar(TypeKind::String))));
  assert!(!is_cow_field(&td_defined("test.Foo")));
}
