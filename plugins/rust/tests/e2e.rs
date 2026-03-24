//! End-to-end tests that invoke the bebopc-gen-rust binary via stdin/stdout.

use std::borrow::Cow;
use std::process::Command;

use bebop_runtime::{BebopDecode, BebopEncode};

// Import the generated protocol types (they live in the same crate)
#[path = "../src/generated/mod.rs"]
mod generated;
use generated::*;

fn invoke_generator(request: &CodeGeneratorRequest) -> CodeGeneratorResponse<'static> {
  let mut writer = bebop_runtime::BebopWriter::new();
  request.encode(&mut writer);
  let input_bytes = writer.into_bytes();

  let binary = env!("CARGO_BIN_EXE_bebopc-gen-rust");
  let output = Command::new(binary)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .and_then(|mut child| {
      use std::io::Write;
      child.stdin.take().unwrap().write_all(&input_bytes)?;
      child.wait_with_output()
    })
    .expect("failed to run bebopc-gen-rust");

  assert!(
    output.status.success(),
    "generator failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let mut reader = bebop_runtime::BebopReader::new(&output.stdout);
  CodeGeneratorResponse::decode(&mut reader)
    .expect("failed to decode response")
    .into_owned()
}

fn make_request(parameter: Option<&str>) -> CodeGeneratorRequest<'static> {
  let schema_path = "/test/test_types.bop";
  CodeGeneratorRequest {
    files_to_generate: Some(vec![Cow::Owned(schema_path.to_string())]),
    parameter: parameter.map(|s| Cow::Owned(s.to_string())),
    compiler_version: None,
    schemas: Some(vec![SchemaDescriptor {
      path: Some(Cow::Owned(schema_path.to_string())),
      definitions: Some(vec![DefinitionDescriptor {
        kind: Some(DefinitionKind::Struct),
        name: Some(Cow::Borrowed("Point")),
        fqn: Some(Cow::Borrowed("test.Point")),
        struct_def: Some(StructDef {
          fields: Some(vec![FieldDescriptor {
            name: Some(Cow::Borrowed("x")),
            r#type: Some(TypeDescriptor {
              kind: Some(TypeKind::Float32),
              ..Default::default()
            }),
            index: Some(0),
            ..Default::default()
          }]),
          fixed_size: Some(4),
          ..Default::default()
        }),
        ..Default::default()
      }]),
      ..Default::default()
    }]),
    host_options: None,
  }
}

#[test]
fn e2e_serde_always() {
  let request = make_request(Some("serde"));
  let response = invoke_generator(&request);

  assert!(
    response.error.is_none(),
    "generator error: {:?}",
    response.error
  );
  let files = response.files.expect("should have generated files");
  assert_eq!(files.len(), 1);
  let code = files[0]
    .content
    .as_deref()
    .expect("file should have content");

  assert!(
    code.contains("#[derive(serde::Serialize, serde::Deserialize)]"),
    "should contain unconditional serde derive"
  );
  assert!(!code.contains("cfg_attr"), "should not contain cfg_attr");
  assert!(
    code.contains("use bebop_runtime::serde;"),
    "should contain unconditional serde import"
  );
}

#[test]
fn e2e_serde_feature_gated() {
  let request = make_request(Some("serde-feature:my_feat"));
  let response = invoke_generator(&request);

  assert!(response.error.is_none());
  let files = response.files.expect("should have generated files");
  let code = files[0].content.as_deref().unwrap();

  assert!(
    code
      .contains("#[cfg_attr(feature = \"my_feat\", derive(serde::Serialize, serde::Deserialize))]"),
    "should contain feature-gated serde derive"
  );
  assert!(
    code.contains("#[cfg(feature = \"my_feat\")]\nuse bebop_runtime::serde;"),
    "should contain feature-gated serde import"
  );
}

#[test]
fn e2e_serde_disabled() {
  let request = make_request(None);
  let response = invoke_generator(&request);

  assert!(response.error.is_none());
  let files = response.files.expect("should have generated files");
  let code = files[0].content.as_deref().unwrap();

  assert!(
    !code.contains("serde"),
    "should not contain any serde references when disabled"
  );
}
