pub mod field_codegen;
pub mod gen_const;
pub mod gen_enum;
pub mod gen_message;
pub mod gen_service;
pub mod gen_struct;
pub mod gen_union;
pub mod naming;
pub mod schema_analysis;

use std::borrow::Cow;

use crate::error::GeneratorError;
use crate::generated::*;
pub use schema_analysis::SchemaAnalysis;

/// Return the Rust visibility keyword for a definition.
///
/// `Visibility::Local`/`Export` always win; `Default` uses generator options.
pub fn visibility_keyword(def: &DefinitionDescriptor, options: &GeneratorOptions) -> &'static str {
  match def.visibility {
    Some(Visibility::Local) => "pub(crate)",
    Some(Visibility::Export) => "pub",
    Some(Visibility::Default) | None => options.default_visibility.keyword(),
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultVisibility {
  Public,
  Crate,
}

impl DefaultVisibility {
  pub fn keyword(self) -> &'static str {
    match self {
      Self::Public => "pub",
      Self::Crate => "pub(crate)",
    }
  }

  fn parse(value: &str) -> Result<Self, GeneratorError> {
    if value.eq_ignore_ascii_case("public") {
      Ok(Self::Public)
    } else if value.eq_ignore_ascii_case("crate") {
      Ok(Self::Crate)
    } else {
      Err(GeneratorError::InvalidOption(format!(
        "invalid host option Visibility={value}; expected public|crate"
      )))
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerdeMode {
  /// No serde code emitted.
  Disabled,
  /// Unconditional serde derives (no cfg_attr).
  Always,
  /// Wrap serde derives in `cfg_attr(feature = "<name>", ...)`.
  FeatureGated(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorOptions {
  pub default_visibility: DefaultVisibility,
  pub serde: SerdeMode,
}

impl Default for GeneratorOptions {
  fn default() -> Self {
    Self {
      default_visibility: DefaultVisibility::Public,
      serde: SerdeMode::Disabled,
    }
  }
}

impl GeneratorOptions {
  pub fn new(
    host_options: Option<&bebop_runtime::HashMap<Cow<'_, str>, Cow<'_, str>>>,
    parameter: Option<&str>,
  ) -> Result<Self, GeneratorError> {
    let mut options = Self::default();

    // Host options: Visibility
    if let Some(host_options) = host_options {
      if let Some(value) = host_options.get("Visibility") {
        options.default_visibility = DefaultVisibility::parse(value.as_ref())?;
      }
    }

    // Plugin parameter: serde, serde-feature:<name>
    if let Some(param) = parameter {
      for token in param.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if token == "serde" {
          options.serde = SerdeMode::Always;
        } else if let Some(feat) = token.strip_prefix("serde-feature:") {
          options.serde = SerdeMode::FeatureGated(feat.to_string());
        } else {
          return Err(GeneratorError::InvalidOption(format!(
            "unknown plugin option: {:?}",
            token
          )));
        }
      }
    }

    Ok(options)
  }
}

impl SerdeMode {
  pub fn emit_derive(&self, output: &mut String) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "#[cfg_attr(feature = \"{}\", derive(serde::Serialize, serde::Deserialize))]\n",
          feat
        ));
      }
    }
  }

  pub fn emit_field_attr(&self, output: &mut String, attr: &str) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str(&format!("  #[serde({})]\n", attr));
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "  #[cfg_attr(feature = \"{}\", serde({}))]\n",
          feat, attr
        ));
      }
    }
  }

  pub fn emit_type_attr(&self, output: &mut String, attr: &str) {
    match self {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str(&format!("#[serde({})]\n", attr));
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "#[cfg_attr(feature = \"{}\", serde({}))]\n",
          feat, attr
        ));
      }
    }
  }
}

pub struct RustGenerator {
  pub compiler_version: Option<VersionOwned>,
  pub options: GeneratorOptions,
}

impl RustGenerator {
  #[allow(dead_code)]
  pub fn new(compiler_version: Option<VersionOwned>) -> Self {
    Self::with_options(compiler_version, GeneratorOptions::default())
  }

  pub fn with_options(compiler_version: Option<VersionOwned>, options: GeneratorOptions) -> Self {
    Self {
      compiler_version,
      options,
    }
  }

  /// Generate Rust code for a single schema.
  ///
  /// `sibling_imports` contains the module stems of other schemas being generated
  /// alongside this one. For each stem, a `use super::{stem}::*;` is emitted so
  /// that cross-module type references resolve correctly.
  pub fn generate(
    &self,
    schema: &SchemaDescriptor,
    sibling_imports: &[&str],
    analysis: &SchemaAnalysis,
  ) -> Result<String, GeneratorError> {
    let mut output = String::new();

    let definitions = schema.definitions.as_deref().unwrap_or(&[]);

    // File header
    output.push_str("// This file is @generated by bebopc-gen-rust. DO NOT EDIT.\n");
    if let Some(ref path) = schema.path {
      output.push_str(&format!("// source: {}\n", path));
    }
    if let Some(v) = &self.compiler_version {
      output.push_str(&format!("// bebopc {}\n", v));
    }
    if let Some(ref edition) = schema.edition {
      if *edition == Edition::Edition2026 {
        output.push_str("// edition 2026\n");
      }
    }
    output.push_str("//\n");
    output.push_str("// This file requires the Bebop runtime library.\n");
    output.push_str("// https://github.com/6over3/bebop\n");
    output.push_str("//\n");
    output.push_str("// SPDX-License-Identifier: Apache-2.0\n");
    output.push_str("// Copyright (c) 6OVER3 INSTITUTE\n\n");

    // Inner attributes — scoped to this module only
    output.push_str("#![allow(warnings)]\n");
    output.push_str("#![no_implicit_prelude]\n\n");
    output.push_str("extern crate alloc;\n");
    output.push_str("extern crate bebop_runtime;\n");
    output.push_str("extern crate core;\n");
    // Import modules (not types) so generated identifiers never shadow user-defined Bebop
    // types. User types are PascalCase; these are all lowercase module names.
    output.push_str("use alloc::{borrow, boxed, string, vec};\n");
    output.push_str("use bebop_runtime as bebop;\n");
    output.push_str("use bebop_runtime::DecodeContext as _;\n");
    output.push_str("use core::convert::Into as _;\n");
    output.push_str("use core::iter::{IntoIterator as _, Iterator as _};\n");
    output.push_str("use core::{convert, default, iter, mem, ops, option, result};\n");
    match &self.options.serde {
      SerdeMode::Disabled => {}
      SerdeMode::Always => {
        output.push_str("use bebop_runtime::serde;\n");
      }
      SerdeMode::FeatureGated(feat) => {
        output.push_str(&format!(
          "#[cfg(feature = \"{}\")]\nuse bebop_runtime::serde;\n",
          feat
        ));
      }
    }

    // Cross-module imports for sibling schemas
    for stem in sibling_imports {
      output.push_str(&format!("use super::{}::*;\n", stem));
    }
    output.push('\n');
    output.push_str("// @@bebop_insertion_point(imports)\n\n");

    for def in definitions {
      self.generate_definition(def, &mut output, 0, analysis)?;
    }

    output.push_str("// @@bebop_insertion_point(eof)\n");

    Ok(output)
  }

  fn generate_definition(
    &self,
    def: &DefinitionDescriptor,
    output: &mut String,
    depth: usize,
    analysis: &SchemaAnalysis,
  ) -> Result<(), GeneratorError> {
    let kind = def
      .kind
      .ok_or_else(|| GeneratorError::MalformedDefinition("definition missing kind".into()))?;
    let name = def.name.as_deref().unwrap_or("<unnamed>");
    let fqn = def.fqn.as_deref().unwrap_or("<no fqn>");

    // Log what we're processing
    let indent = "  ".repeat(depth);
    eprintln!(
      "[bebopc-gen-rust] {}{} {} ({})",
      indent,
      kind.name(),
      name,
      fqn,
    );

    match kind {
      DefinitionKind::Enum => gen_enum::generate(def, output, &self.options)?,
      DefinitionKind::Struct => gen_struct::generate(def, output, &self.options, analysis)?,
      DefinitionKind::Message => gen_message::generate(def, output, &self.options, analysis)?,
      DefinitionKind::Union => gen_union::generate(def, output, &self.options, analysis)?,
      DefinitionKind::Const => gen_const::generate(def, output, &self.options)?,
      DefinitionKind::Service => gen_service::generate(def, output)?,
      DefinitionKind::Decorator => { /* Skip decorator definitions */ }
      DefinitionKind::Unknown => {
        eprintln!(
          "[bebopc-gen-rust] {}  WARNING: unknown definition kind",
          indent
        );
      }
    }

    // Process nested definitions
    if let Some(ref nested) = def.nested {
      for child in nested {
        self.generate_definition(child, output, depth + 1, analysis)?;
      }
    }

    Ok(())
  }
}

/// Emit `///` doc comment lines if documentation is present.
pub fn emit_doc_comment(output: &mut String, doc: &Option<Cow<'_, str>>) {
  if let Some(ref text) = *doc {
    for line in text.lines() {
      output.push_str(&format!("/// {}\n", line));
    }
  }
}

/// Tests whether a decorator FQN matches an expected name.
/// Handles both bare names (`"deprecated"`) and fully-qualified names
/// (`"bebop.deprecated"`) because the compiler currently sends the
/// as-written name rather than the resolved FQN.
fn decorator_matches(fqn: &str, expected: &str) -> bool {
  fqn == expected
    || (expected.len() > fqn.len()
      && expected.ends_with(fqn)
      && expected.as_bytes()[expected.len() - fqn.len() - 1] == b'.')
}

/// Fully-qualified name for the `@deprecated` decorator.
pub const DEPRECATED: &str = "bebop.deprecated";

/// Emit `#[deprecated]` attribute if the decorators contain `@deprecated`.
pub fn emit_deprecated(output: &mut String, decorators: &Option<Vec<DecoratorUsage<'_>>>) {
  if let Some(ref decs) = *decorators {
    for dec in decs {
      if dec
        .fqn
        .as_deref()
        .is_some_and(|fqn| decorator_matches(fqn, DEPRECATED))
      {
        // Check for a message argument
        let msg = dec.args.as_ref().and_then(|args| {
          args.iter().find_map(|arg| {
            if let crate::generated::LiteralValue {
              string_value: Some(ref s),
              ..
            } = arg.value
            {
              Some(s.clone())
            } else {
              None
            }
          })
        });
        if let Some(msg) = msg {
          output.push_str(&format!("#[deprecated(note = \"{}\")]\n", msg));
        } else {
          output.push_str("#[deprecated]\n");
        }
        return;
      }
    }
  }
}

/// Fully-qualified name for the `@forward_compatible` decorator.
pub const FORWARD_COMPATIBLE: &str = "bebop.forward_compatible";

/// Returns `true` if the definition has a decorator matching `name`.
/// Matches both bare names (`"forward_compatible"`) and fully-qualified
/// names (`"bebop.forward_compatible"`).
pub fn has_decorator(def: &DefinitionDescriptor, name: &str) -> bool {
  def.decorators.as_ref().is_some_and(|decs| {
    decs.iter().any(|d| {
      d.fqn
        .as_deref()
        .is_some_and(|fqn| decorator_matches(fqn, name))
    })
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::borrow::Cow;

  use crate::generated::{
    ConstDef, DecoratorUsage, DefinitionDescriptor, DefinitionKind, EnumDef, EnumMemberDescriptor,
    FieldDescriptor, LiteralKind, LiteralValue, MessageDef, SchemaDescriptor, StructDef,
    TypeDescriptor, TypeKind, UnionBranchDescriptor, UnionDef, Visibility,
  };

  fn scalar_type(kind: TypeKind) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(kind),
      ..Default::default()
    }
  }

  fn array_type(element: TypeDescriptor<'static>) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Array),
      array_element: Some(Box::new(element)),
      ..Default::default()
    }
  }

  fn defined_type(fqn: &'static str) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Defined),
      defined_fqn: Some(Cow::Borrowed(fqn)),
      ..Default::default()
    }
  }

  fn build_schema() -> SchemaDescriptor<'static> {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Msg")),
      fqn: Some(Cow::Borrowed("test.Msg")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let status = DefinitionDescriptor {
      kind: Some(DefinitionKind::Enum),
      name: Some(Cow::Borrowed("Status")),
      fqn: Some(Cow::Borrowed("test.Status")),
      enum_def: Some(EnumDef {
        base_type: Some(TypeKind::Uint32),
        members: Some(vec![
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("UNKNOWN")),
            value: Some(0),
            ..Default::default()
          },
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("OK")),
            value: Some(1),
            ..Default::default()
          },
        ]),
        is_flags: Some(false),
      }),
      ..Default::default()
    };

    let result_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("ResultUnion")),
      fqn: Some(Cow::Borrowed("test.ResultUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![payload, msg, status, result_union]),
      ..Default::default()
    }
  }

  fn assert_order(output: &str, first: &str, second: &str) {
    let first_idx = output
      .find(first)
      .unwrap_or_else(|| panic!("missing insertion point: {}", first));
    let second_idx = output
      .find(second)
      .unwrap_or_else(|| panic!("missing insertion point: {}", second));
    assert!(
      first_idx < second_idx,
      "expected {} to appear before {}",
      first,
      second
    );
  }

  fn build_string_array_constructor_schema() -> SchemaDescriptor<'static> {
    let token_batch = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("TokenBatch")),
      fqn: Some(Cow::Borrowed("TokenBatch")),
      struct_def: Some(StructDef {
        fields: Some(vec![
          FieldDescriptor {
            name: Some(Cow::Borrowed("id")),
            r#type: Some(scalar_type(TypeKind::Uint32)),
            index: Some(0),
            ..Default::default()
          },
          FieldDescriptor {
            name: Some(Cow::Borrowed("tokens")),
            r#type: Some(array_type(scalar_type(TypeKind::String))),
            index: Some(1),
            ..Default::default()
          },
        ]),
        ..Default::default()
      }),
      ..Default::default()
    };

    SchemaDescriptor {
      path: Some(Cow::Borrowed("constructor.bop")),
      definitions: Some(vec![token_batch]),
      ..Default::default()
    }
  }

  #[test]
  fn emits_file_level_insertion_points() {
    let schema = build_schema();
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(output.contains("// @@bebop_insertion_point(imports)"));
    assert!(output.contains("// @@bebop_insertion_point(eof)"));
    assert_order(
      &output,
      "// @@bebop_insertion_point(imports)",
      "// @@bebop_insertion_point(eof)",
    );
  }

  #[test]
  fn emits_type_and_codec_insertion_points() {
    let schema = build_schema();
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    for marker in [
      "// @@bebop_insertion_point(struct_scope:Payload)",
      "// @@bebop_insertion_point(message_scope:Msg)",
      "// @@bebop_insertion_point(enum_scope:Status)",
      "// @@bebop_insertion_point(union_scope:ResultUnion)",
      "// @@bebop_insertion_point(encode_start:Payload)",
      "// @@bebop_insertion_point(encode_end:Payload)",
      "// @@bebop_insertion_point(decode_start:Payload)",
      "// @@bebop_insertion_point(decode_end:Payload)",
      "// @@bebop_insertion_point(encode_start:Msg)",
      "// @@bebop_insertion_point(encode_end:Msg)",
      "// @@bebop_insertion_point(decode_start:Msg)",
      "// @@bebop_insertion_point(decode_end:Msg)",
      "// @@bebop_insertion_point(encode_start:Status)",
      "// @@bebop_insertion_point(encode_end:Status)",
      "// @@bebop_insertion_point(decode_start:Status)",
      "// @@bebop_insertion_point(decode_end:Status)",
      "// @@bebop_insertion_point(encode_start:ResultUnion)",
      "// @@bebop_insertion_point(encode_switch:ResultUnion)",
      "// @@bebop_insertion_point(encode_end:ResultUnion)",
      "// @@bebop_insertion_point(decode_start:ResultUnion)",
      "// @@bebop_insertion_point(decode_switch:ResultUnion)",
      "// @@bebop_insertion_point(decode_end:ResultUnion)",
    ] {
      assert!(
        output.contains(marker),
        "missing insertion point: {}",
        marker
      );
    }

    assert_order(
      &output,
      "// @@bebop_insertion_point(encode_start:ResultUnion)",
      "// @@bebop_insertion_point(encode_switch:ResultUnion)",
    );
    assert_order(
      &output,
      "// @@bebop_insertion_point(encode_switch:ResultUnion)",
      "// @@bebop_insertion_point(encode_end:ResultUnion)",
    );
    assert_order(
      &output,
      "// @@bebop_insertion_point(decode_start:ResultUnion)",
      "// @@bebop_insertion_point(decode_switch:ResultUnion)",
    );
    assert_order(
      &output,
      "// @@bebop_insertion_point(decode_switch:ResultUnion)",
      "// @@bebop_insertion_point(decode_end:ResultUnion)",
    );
  }

  #[test]
  fn struct_constructor_converts_owned_string_arrays() {
    let schema = build_string_array_constructor_schema();
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(
      output.contains("impl iter::IntoIterator<Item = impl convert::Into<borrow::Cow<'buf, str>>>"),
      "expected IntoIterator param for string array; output:\n{}",
      output
    );
    assert!(
      output.contains("let tokens = tokens.into_iter().map(|_e| _e.into()).collect();"),
      "expected IntoIterator collect expression for string array; output:\n{}",
      output
    );
  }

  #[test]
  fn parses_visibility_host_option() {
    let mut host_options = bebop_runtime::HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
    let options = GeneratorOptions::new(Some(&host_options), None).unwrap();
    assert_eq!(options.default_visibility, DefaultVisibility::Crate);
  }

  #[test]
  fn rejects_invalid_visibility_host_option() {
    let mut host_options = bebop_runtime::HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("private"));
    let err = GeneratorOptions::new(Some(&host_options), None).unwrap_err();
    assert!(matches!(err, GeneratorError::InvalidOption(_)));
  }

  #[test]
  fn applies_default_visibility_option_to_implicit_visibility() {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      // `None` should behave like `Visibility::Default`.
      visibility: None,
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("default_visibility.bop")),
      definitions: Some(vec![payload]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        default_visibility: DefaultVisibility::Crate,
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(output.contains("pub(crate) struct Payload"));
    assert!(output.contains("  pub(crate) id: i32,"));
  }

  #[test]
  fn emits_pub_crate_for_local_visibility() {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      visibility: Some(Visibility::Local),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Msg")),
      fqn: Some(Cow::Borrowed("test.Msg")),
      visibility: Some(Visibility::Local),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let status = DefinitionDescriptor {
      kind: Some(DefinitionKind::Enum),
      name: Some(Cow::Borrowed("Status")),
      fqn: Some(Cow::Borrowed("test.Status")),
      visibility: Some(Visibility::Local),
      enum_def: Some(EnumDef {
        base_type: Some(TypeKind::Uint32),
        members: Some(vec![EnumMemberDescriptor {
          name: Some(Cow::Borrowed("OK")),
          value: Some(1),
          ..Default::default()
        }]),
        is_flags: Some(false),
      }),
      ..Default::default()
    };

    let flags = DefinitionDescriptor {
      kind: Some(DefinitionKind::Enum),
      name: Some(Cow::Borrowed("Perms")),
      fqn: Some(Cow::Borrowed("test.Perms")),
      visibility: Some(Visibility::Local),
      enum_def: Some(EnumDef {
        base_type: Some(TypeKind::Uint32),
        members: Some(vec![EnumMemberDescriptor {
          name: Some(Cow::Borrowed("READ")),
          value: Some(1),
          ..Default::default()
        }]),
        is_flags: Some(true),
      }),
      ..Default::default()
    };

    let result_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("ResultUnion")),
      fqn: Some(Cow::Borrowed("test.ResultUnion")),
      visibility: Some(Visibility::Local),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("local.bop")),
      definitions: Some(vec![payload, msg, status, flags, result_union]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Struct: definition and fields
    assert!(
      output.contains("pub(crate) struct Payload"),
      "struct should use pub(crate)"
    );
    assert!(
      output.contains("  pub(crate) id: i32,"),
      "struct field should use pub(crate)"
    );

    // Message: definition and fields
    assert!(
      output.contains("pub(crate) struct Msg"),
      "message should use pub(crate)"
    );
    assert!(
      output.contains("  pub(crate) id: option::Option<i32>,"),
      "message field should use pub(crate)"
    );

    // Enum
    assert!(
      output.contains("pub(crate) enum Status"),
      "enum should use pub(crate)"
    );

    // Flags: struct and inner field
    assert!(
      output.contains("pub(crate) struct Perms(pub(crate) u32)"),
      "flags struct should use pub(crate)"
    );

    // Union: definition and type alias
    assert!(
      output.contains("pub(crate) enum ResultUnion<'buf>"),
      "union should use pub(crate)"
    );
    assert!(
      output.contains("pub(crate) type ResultUnionOwned"),
      "union type alias should use pub(crate)"
    );

    // Verify no bare `pub ` (without `(crate)`) on definitions
    assert!(
      !output.contains("\npub struct "),
      "should not contain bare `pub struct`"
    );
    assert!(
      !output.contains("\npub enum "),
      "should not contain bare `pub enum`"
    );
    assert!(
      !output.contains("\npub type "),
      "should not contain bare `pub type`"
    );
  }

  #[test]
  fn emits_pub_for_export_visibility() {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      visibility: Some(Visibility::Export),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("export.bop")),
      definitions: Some(vec![payload]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(
      output.contains("pub struct Payload"),
      "Export visibility should emit pub"
    );
    assert!(
      output.contains("  pub id: i32,"),
      "Export visibility fields should emit pub"
    );
  }

  #[test]
  fn emits_pub_crate_for_local_const() {
    let my_const = DefinitionDescriptor {
      kind: Some(DefinitionKind::Const),
      name: Some(Cow::Borrowed("MY_VALUE")),
      fqn: Some(Cow::Borrowed("test.MY_VALUE")),
      visibility: Some(Visibility::Local),
      const_def: Some(ConstDef {
        r#type: Some(scalar_type(TypeKind::Int32)),
        value: Some(LiteralValue {
          kind: Some(LiteralKind::Int),
          int_value: Some(42),
          ..Default::default()
        }),
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("const.bop")),
      definitions: Some(vec![my_const]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(
      output.contains("pub(crate) const MY_VALUE: i32 = 42i32;"),
      "local const should use pub(crate), got: {}",
      output
    );
  }

  /// Message `with_*` setters for defined-type fields with a lifetime must
  /// accept `T<'buf>`, not `T<'static>`. The setter stores the value directly
  /// into an `Option<T<'buf>>` field — no ownership conversion is needed.
  /// Struct constructors intentionally use `T<'static>` (owned form) and rely
  /// on covariance, but message setters should not impose that restriction.
  #[test]
  fn message_setter_uses_buf_lifetime_for_defined_type_fields() {
    // Inner is a struct with a string field → needs 'buf
    let inner = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Inner")),
      fqn: Some(Cow::Borrowed("test.Inner")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("name")),
          r#type: Some(scalar_type(TypeKind::String)),
          index: Some(0),
          ..Default::default()
        }]),
        ..Default::default()
      }),
      ..Default::default()
    };

    // Wrapper is a message with a field of type Inner
    let wrapper = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Wrapper")),
      fqn: Some(Cow::Borrowed("test.Wrapper")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("payload")),
          r#type: Some(defined_type("test.Inner")),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("msg-setter.bop")),
      definitions: Some(vec![inner, wrapper]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // The setter must accept Inner<'buf>, not Inner<'static>
    assert!(
      output.contains("pub fn with_payload(mut self, value: Inner<'buf>) -> Self"),
      "message setter should use 'buf lifetime for defined-type field;\n\
       look for 'with_payload' in output:\n{}",
      output
    );
  }

  fn forward_compatible_decorator() -> Vec<DecoratorUsage<'static>> {
    vec![DecoratorUsage {
      fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
      ..Default::default()
    }]
  }

  #[test]
  fn forward_compatible_enum_emits_unknown_variant() {
    let fc_enum = DefinitionDescriptor {
      kind: Some(DefinitionKind::Enum),
      name: Some(Cow::Borrowed("Color")),
      fqn: Some(Cow::Borrowed("test.Color")),
      enum_def: Some(EnumDef {
        base_type: Some(TypeKind::Uint32),
        members: Some(vec![
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("Red")),
            value: Some(1),
            ..Default::default()
          },
          EnumMemberDescriptor {
            name: Some(Cow::Borrowed("Green")),
            value: Some(2),
            ..Default::default()
          },
        ]),
        is_flags: Some(false),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-enum.bop")),
      definitions: Some(vec![fc_enum]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should NOT have #[repr]
    assert!(!output.contains("#[repr("));
    // Should have Unknown variant
    assert!(output.contains("Unknown(u32)"));
    // Should have discriminator() method
    assert!(output.contains("fn discriminator(self)"));
    // Should have is_known() method
    assert!(output.contains("fn is_known("));
    // Should use From, not TryFrom
    assert!(output.contains("impl convert::From<u32> for Color"));
    assert!(!output.contains("TryFrom"));
    // Should use discriminator() in encode
    assert!(output.contains("self.discriminator()"));
    // Decode should use From
    assert!(output.contains("convert::From<_>>::from(value)"));
  }

  #[test]
  fn strict_union_rejects_unknown_discriminator() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let strict_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("StrictUnion")),
      fqn: Some(Cow::Borrowed("test.StrictUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict-union.bop")),
      definitions: Some(vec![inner_struct, strict_union]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should NOT have Unknown variant
    assert!(!output.contains("Unknown(u8,"));
    // Should NOT have 'buf on the enum (all branches are scalar structs)
    assert!(output.contains("pub enum StrictUnion {"));
    assert!(!output.contains("pub enum StrictUnion<'buf>"));
    // Should have InvalidUnion error in decode
    assert!(output.contains("InvalidUnion"));
  }

  #[test]
  fn forward_compatible_union_has_unknown_variant() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let fc_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("FcUnion")),
      fqn: Some(Cow::Borrowed("test.FcUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("payload")),
          type_ref_fqn: Some(Cow::Borrowed("test.Payload")),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-union.bop")),
      definitions: Some(vec![inner_struct, fc_union]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have Unknown variant with Cow
    assert!(output.contains("Unknown(u8, borrow::Cow<'buf, [u8]>)"));
    // Should have 'buf
    assert!(output.contains("pub enum FcUnion<'buf>"));
    // Should NOT have InvalidUnion
    assert!(!output.contains("InvalidUnion"));
  }

  #[test]
  fn strict_message_rejects_unknown_field_tag() {
    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("StrictMsg")),
      fqn: Some(Cow::Borrowed("test.StrictMsg")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict-msg.bop")),
      definitions: Some(vec![msg]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have InvalidField error
    assert!(output.contains("InvalidField"));
    // Should NOT have reader.skip
    assert!(!output.contains("reader.skip("));
  }

  #[test]
  fn forward_compatible_message_skips_unknown_fields() {
    let msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("FcMsg")),
      fqn: Some(Cow::Borrowed("test.FcMsg")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc-msg.bop")),
      definitions: Some(vec![msg]),
      ..Default::default()
    };

    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have reader.skip (forward-compatible behavior)
    assert!(output.contains("reader.skip("));
    // Should NOT have InvalidField
    assert!(!output.contains("InvalidField"));
  }

  #[test]
  fn serde_always_emits_unconditional_struct_derive() {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![payload]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        serde: SerdeMode::Always,
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(
      output.contains("#[derive(serde::Serialize, serde::Deserialize)]\n#[derive(Debug"),
      "should emit unconditional serde derive, got:\n{}",
      output
    );
    assert!(!output.contains("cfg_attr"), "should not contain cfg_attr");
  }

  #[test]
  fn serde_disabled_omits_struct_derive() {
    let payload = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Payload")),
      fqn: Some(Cow::Borrowed("test.Payload")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("id")),
          r#type: Some(scalar_type(TypeKind::Int32)),
          index: Some(0),
          ..Default::default()
        }]),
        fixed_size: Some(4),
        ..Default::default()
      }),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![payload]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        serde: SerdeMode::Disabled,
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(
      !output.contains("serde"),
      "should not contain any serde references"
    );
  }

  #[test]
  fn serde_always_emits_unconditional_import() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        serde: SerdeMode::Always,
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(output.contains("use bebop_runtime::serde;\n"));
    assert!(!output.contains("cfg(feature"));
  }

  #[test]
  fn serde_feature_gated_emits_cfg_import() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        serde: SerdeMode::FeatureGated(String::from("my-serde")),
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(output.contains("#[cfg(feature = \"my-serde\")]\nuse bebop_runtime::serde;\n"));
  }

  #[test]
  fn serde_disabled_omits_import() {
    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("test.bop")),
      definitions: Some(vec![]),
      ..Default::default()
    };
    let analysis = SchemaAnalysis::build(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        serde: SerdeMode::Disabled,
        ..Default::default()
      },
    )
    .generate(&schema, &[], &analysis)
    .expect("generator should succeed");

    assert!(!output.contains("bebop_runtime::serde"));
  }

  #[test]
  fn serde_defaults_to_disabled() {
    let options = GeneratorOptions::new(None, None).unwrap();
    assert_eq!(options.serde, SerdeMode::Disabled);
  }

  #[test]
  fn parses_serde_from_parameter() {
    let options = GeneratorOptions::new(None, Some("serde")).unwrap();
    assert_eq!(options.serde, SerdeMode::Always);
  }

  #[test]
  fn parses_serde_feature_from_parameter() {
    let options = GeneratorOptions::new(None, Some("serde-feature:my_feat")).unwrap();
    assert_eq!(
      options.serde,
      SerdeMode::FeatureGated(String::from("my_feat"))
    );
  }

  #[test]
  fn parses_serde_among_multiple_params() {
    // Test comma-separated parsing: serde can appear with other serde-prefixed tokens
    let options = GeneratorOptions::new(None, Some("serde-feature:feat1")).unwrap();
    assert_eq!(
      options.serde,
      SerdeMode::FeatureGated(String::from("feat1"))
    );
    // Also test that trimming works around commas
    let options = GeneratorOptions::new(None, Some(" serde ")).unwrap();
    assert_eq!(options.serde, SerdeMode::Always);
  }

  #[test]
  fn rejects_unknown_parameter() {
    let err = GeneratorOptions::new(None, Some("bogus")).unwrap_err();
    assert!(matches!(err, GeneratorError::InvalidOption(_)));
  }

  #[test]
  fn visibility_still_from_host_options() {
    let mut host_options = bebop_runtime::HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
    let options = GeneratorOptions::new(Some(&host_options), None).unwrap();
    assert_eq!(options.default_visibility, DefaultVisibility::Crate);
  }

  #[test]
  fn both_host_options_and_parameter() {
    let mut host_options = bebop_runtime::HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
    let options = GeneratorOptions::new(Some(&host_options), Some("serde")).unwrap();
    assert_eq!(options.default_visibility, DefaultVisibility::Crate);
    assert_eq!(options.serde, SerdeMode::Always);
  }
}
