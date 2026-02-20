pub mod gen_const;
pub mod gen_enum;
pub mod gen_message;
pub mod gen_service;
pub mod gen_struct;
pub mod gen_union;
pub mod naming;
pub mod type_mapper;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use crate::error::GeneratorError;
use crate::generated::*;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorOptions {
  pub default_visibility: DefaultVisibility,
}

impl Default for GeneratorOptions {
  fn default() -> Self {
    Self {
      default_visibility: DefaultVisibility::Public,
    }
  }
}

impl GeneratorOptions {
  pub fn from_host_options(
    host_options: Option<&HashMap<Cow<'_, str>, Cow<'_, str>>>,
  ) -> Result<Self, GeneratorError> {
    let mut options = Self::default();
    if let Some(host_options) = host_options {
      if let Some(value) = host_options.get("Visibility") {
        options.default_visibility = DefaultVisibility::parse(value.as_ref())?;
      }
    }
    Ok(options)
  }
}

/// Pre-computed lifetime and kind information for all definitions in a schema.
pub struct LifetimeAnalysis {
  /// FQNs of enum definitions (never need a lifetime parameter).
  pub enum_fqns: HashSet<String>,
  /// FQNs of types that need `'buf` (contain strings, byte arrays, or unions).
  pub lifetime_fqns: HashSet<String>,
  /// FQNs of types that can derive `Eq` (no floating-point fields transitively).
  pub eq_fqns: HashSet<String>,
  /// FQNs of types that can derive `Hash` (no floating-point or map fields transitively).
  pub hash_fqns: HashSet<String>,
}

impl LifetimeAnalysis {
  /// Build a combined analysis from all schemas, so cross-schema type
  /// references resolve correctly.
  pub fn build_all(schemas: &[SchemaDescriptor]) -> Self {
    let mut analysis = LifetimeAnalysis {
      enum_fqns: HashSet::new(),
      lifetime_fqns: HashSet::new(),
      eq_fqns: HashSet::new(),
      hash_fqns: HashSet::new(),
    };

    let mut def_by_fqn: HashMap<String, &DefinitionDescriptor> = HashMap::new();
    for schema in schemas {
      let definitions = schema.definitions.as_deref().unwrap_or(&[]);
      collect_definitions(definitions, &mut def_by_fqn);
    }

    for (fqn, def) in &def_by_fqn {
      if def.kind == Some(DefinitionKind::Enum) {
        analysis.enum_fqns.insert(fqn.clone());
      }
      if def.kind == Some(DefinitionKind::Union) {
        // Unions always need lifetime (Unknown variant uses Cow<'buf, [u8]>)
        analysis.lifetime_fqns.insert(fqn.clone());
      }
    }

    // Resolve lifetime requirements to a fixpoint so forward references work.
    loop {
      let mut changed = false;
      for (fqn, def) in &def_by_fqn {
        if analysis.lifetime_fqns.contains(fqn) {
          continue;
        }

        let needs_lifetime = match def.kind {
          Some(DefinitionKind::Struct) => def
            .struct_def
            .as_ref()
            .and_then(|sd| sd.fields.as_deref())
            .is_some_and(|fields| {
              fields.iter().any(|f| {
                f.r#type
                  .as_ref()
                  .is_some_and(|td| analysis.type_needs_lifetime(td))
              })
            }),
          Some(DefinitionKind::Message) => def
            .message_def
            .as_ref()
            .and_then(|md| md.fields.as_deref())
            .is_some_and(|fields| {
              fields.iter().any(|f| {
                f.r#type
                  .as_ref()
                  .is_some_and(|td| analysis.type_needs_lifetime(td))
              })
            }),
          Some(DefinitionKind::Union) => true,
          _ => false,
        };

        if needs_lifetime {
          analysis.lifetime_fqns.insert(fqn.clone());
          changed = true;
        }
      }

      if !changed {
        break;
      }
    }
    analysis.analyze_trait_derives(schemas);
    analysis
  }

  /// Check if a TypeDescriptor transitively contains strings or byte arrays.
  fn type_needs_lifetime(&self, td: &TypeDescriptor) -> bool {
    let kind = match td.kind {
      Some(k) => k,
      None => return false,
    };

    match kind {
      TypeKind::String => true,
      TypeKind::Array => {
        if let Some(ref elem) = td.array_element {
          // byte arrays need lifetime (Cow<'buf, [u8]>)
          if elem.kind == Some(TypeKind::Byte) {
            return true;
          }
          self.type_needs_lifetime(elem)
        } else {
          false
        }
      }
      TypeKind::FixedArray => td
        .fixed_array_element
        .as_ref()
        .is_some_and(|e| self.type_needs_lifetime(e)),
      TypeKind::Map => {
        let k = td
          .map_key
          .as_ref()
          .is_some_and(|e| self.type_needs_lifetime(e));
        let v = td
          .map_value
          .as_ref()
          .is_some_and(|e| self.type_needs_lifetime(e));
        k || v
      }
      TypeKind::Defined => {
        if let Some(ref fqn) = td.defined_fqn {
          self.lifetime_fqns.contains(fqn.as_ref())
        } else {
          false
        }
      }
      _ => false,
    }
  }

  /// Returns true when this type can derive `Eq`.
  pub fn can_derive_eq(&self, fqn: &str) -> bool {
    self.eq_fqns.contains(fqn)
  }

  /// Returns true when this type can derive `Hash`.
  pub fn can_derive_hash(&self, fqn: &str) -> bool {
    self.hash_fqns.contains(fqn)
  }

  fn analyze_trait_derives(&mut self, schemas: &[SchemaDescriptor]) {
    #[derive(Clone, Copy, Default)]
    struct TypeTraits {
      has_float: bool,
      has_map: bool,
    }

    impl TypeTraits {
      fn combine(self, other: Self) -> Self {
        Self {
          has_float: self.has_float || other.has_float,
          has_map: self.has_map || other.has_map,
        }
      }
    }

    fn kind_supports_derives(kind: DefinitionKind) -> bool {
      matches!(
        kind,
        DefinitionKind::Enum
          | DefinitionKind::Struct
          | DefinitionKind::Message
          | DefinitionKind::Union
      )
    }

    fn merge_option(acc: TypeTraits, next: Option<TypeTraits>) -> TypeTraits {
      if let Some(next) = next {
        acc.combine(next)
      } else {
        acc
      }
    }

    fn type_traits<'a>(
      td: &TypeDescriptor<'a>,
      def_by_fqn: &HashMap<String, &'a DefinitionDescriptor<'a>>,
      memo: &mut HashMap<String, TypeTraits>,
      visiting: &mut HashSet<String>,
    ) -> TypeTraits {
      let kind = match td.kind {
        Some(k) => k,
        None => return TypeTraits::default(),
      };

      match kind {
        TypeKind::Float16 | TypeKind::Float32 | TypeKind::Float64 | TypeKind::Bfloat16 => {
          TypeTraits {
            has_float: true,
            has_map: false,
          }
        }
        TypeKind::Array => td
          .array_element
          .as_deref()
          .map(|e| type_traits(e, def_by_fqn, memo, visiting))
          .unwrap_or_default(),
        TypeKind::FixedArray => td
          .fixed_array_element
          .as_deref()
          .map(|e| type_traits(e, def_by_fqn, memo, visiting))
          .unwrap_or_default(),
        TypeKind::Map => {
          let mut traits = TypeTraits {
            has_float: false,
            has_map: true,
          };
          traits = merge_option(
            traits,
            td.map_key
              .as_deref()
              .map(|k| type_traits(k, def_by_fqn, memo, visiting)),
          );
          merge_option(
            traits,
            td.map_value
              .as_deref()
              .map(|v| type_traits(v, def_by_fqn, memo, visiting)),
          )
        }
        TypeKind::Defined => td
          .defined_fqn
          .as_deref()
          .map(|fqn| def_traits(fqn, def_by_fqn, memo, visiting))
          .unwrap_or(TypeTraits {
            has_float: true,
            has_map: true,
          }),
        _ => TypeTraits::default(),
      }
    }

    fn def_traits<'a>(
      fqn: &str,
      def_by_fqn: &HashMap<String, &'a DefinitionDescriptor<'a>>,
      memo: &mut HashMap<String, TypeTraits>,
      visiting: &mut HashSet<String>,
    ) -> TypeTraits {
      if let Some(t) = memo.get(fqn) {
        return *t;
      }

      if !visiting.insert(fqn.to_string()) {
        // Recursive reference cycle: treat edge as neutral to avoid infinite recursion.
        return TypeTraits::default();
      }

      let traits = if let Some(def) = def_by_fqn.get(fqn) {
        match def.kind {
          Some(DefinitionKind::Enum) => TypeTraits::default(),
          Some(DefinitionKind::Struct) => def
            .struct_def
            .as_ref()
            .and_then(|sd| sd.fields.as_deref())
            .map(|fields| {
              fields.iter().fold(TypeTraits::default(), |acc, field| {
                let field_traits = field
                  .r#type
                  .as_ref()
                  .map(|td| type_traits(td, def_by_fqn, memo, visiting))
                  .unwrap_or_default();
                acc.combine(field_traits)
              })
            })
            .unwrap_or_default(),
          Some(DefinitionKind::Message) => def
            .message_def
            .as_ref()
            .and_then(|md| md.fields.as_deref())
            .map(|fields| {
              fields.iter().fold(TypeTraits::default(), |acc, field| {
                let field_traits = field
                  .r#type
                  .as_ref()
                  .map(|td| type_traits(td, def_by_fqn, memo, visiting))
                  .unwrap_or_default();
                acc.combine(field_traits)
              })
            })
            .unwrap_or_default(),
          Some(DefinitionKind::Union) => def
            .union_def
            .as_ref()
            .and_then(|ud| ud.branches.as_deref())
            .map(|branches| {
              branches.iter().fold(TypeTraits::default(), |acc, branch| {
                let branch_traits = branch
                  .type_ref_fqn
                  .as_deref()
                  .or(branch.inline_fqn.as_deref())
                  .map(|branch_fqn| def_traits(branch_fqn, def_by_fqn, memo, visiting))
                  .unwrap_or_default();
                acc.combine(branch_traits)
              })
            })
            .unwrap_or_default(),
          _ => TypeTraits::default(),
        }
      } else {
        // Unknown definition reference: be conservative and avoid Eq/Hash derives.
        TypeTraits {
          has_float: true,
          has_map: true,
        }
      };

      visiting.remove(fqn);
      memo.insert(fqn.to_string(), traits);
      traits
    }

    let mut def_by_fqn: HashMap<String, &DefinitionDescriptor> = HashMap::new();
    for schema in schemas {
      let defs = schema.definitions.as_deref().unwrap_or(&[]);
      collect_definitions(defs, &mut def_by_fqn);
    }

    let mut memo: HashMap<String, TypeTraits> = HashMap::new();
    let mut visiting: HashSet<String> = HashSet::new();

    for (fqn, def) in &def_by_fqn {
      let kind = match def.kind {
        Some(k) if kind_supports_derives(k) => k,
        _ => continue,
      };
      let traits = def_traits(fqn, &def_by_fqn, &mut memo, &mut visiting);

      if !traits.has_float {
        self.eq_fqns.insert(fqn.clone());
      }
      if !traits.has_float && !traits.has_map {
        self.hash_fqns.insert(fqn.clone());
      }

      if kind == DefinitionKind::Enum {
        self.eq_fqns.insert(fqn.clone());
        self.hash_fqns.insert(fqn.clone());
      }
    }
  }
}

fn collect_definitions<'a>(
  defs: &'a [DefinitionDescriptor<'a>],
  def_by_fqn: &mut HashMap<String, &'a DefinitionDescriptor<'a>>,
) {
  for def in defs {
    if let Some(fqn) = def.fqn.as_deref() {
      def_by_fqn.insert(fqn.to_string(), def);
    }
    if let Some(nested) = def.nested.as_deref() {
      collect_definitions(nested, def_by_fqn);
    }
  }
}

fn schema_uses_maps(schema: &SchemaDescriptor) -> bool {
  schema
    .definitions
    .as_deref()
    .unwrap_or(&[])
    .iter()
    .any(definition_uses_maps)
}

fn definition_uses_maps(def: &DefinitionDescriptor) -> bool {
  def
    .struct_def
    .as_ref()
    .and_then(|sd| sd.fields.as_deref())
    .is_some_and(fields_use_maps)
    || def
      .message_def
      .as_ref()
      .and_then(|md| md.fields.as_deref())
      .is_some_and(fields_use_maps)
    || def
      .service_def
      .as_ref()
      .and_then(|sd| sd.methods.as_deref())
      .is_some_and(|methods| methods.iter().any(method_uses_maps))
    || def
      .const_def
      .as_ref()
      .and_then(|cd| cd.r#type.as_ref())
      .is_some_and(type_uses_maps)
    || def
      .nested
      .as_deref()
      .is_some_and(|nested| nested.iter().any(definition_uses_maps))
}

fn fields_use_maps(fields: &[FieldDescriptor]) -> bool {
  fields.iter().any(field_uses_maps)
}

fn field_uses_maps(field: &FieldDescriptor) -> bool {
  field.r#type.as_ref().is_some_and(type_uses_maps)
}

fn method_uses_maps(method: &MethodDescriptor) -> bool {
  method.request_type.as_ref().is_some_and(type_uses_maps)
    || method.response_type.as_ref().is_some_and(type_uses_maps)
}

fn type_uses_maps(ty: &TypeDescriptor) -> bool {
  ty.kind == Some(TypeKind::Map)
    || ty.array_element.as_deref().is_some_and(type_uses_maps)
    || ty
      .fixed_array_element
      .as_deref()
      .is_some_and(type_uses_maps)
    || ty.map_key.as_deref().is_some_and(type_uses_maps)
    || ty.map_value.as_deref().is_some_and(type_uses_maps)
}

pub struct RustGenerator {
  pub compiler_version: Option<VersionOwned>,
  pub options: GeneratorOptions,
}

impl RustGenerator {
  pub fn new(compiler_version: Option<VersionOwned>) -> Self {
    Self::with_options(compiler_version, GeneratorOptions::default())
  }

  pub fn with_options(
    compiler_version: Option<VersionOwned>,
    options: GeneratorOptions,
  ) -> Self {
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
    analysis: &LifetimeAnalysis,
  ) -> Result<String, GeneratorError> {
    let mut output = String::new();

    let definitions = schema.definitions.as_deref().unwrap_or(&[]);

    // File header
    output.push_str("// Code generated by bebopc-gen-rust. DO NOT EDIT.\n");
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
    output.push_str("#![allow(warnings)]\n\n");
    output.push_str("extern crate alloc;\n");
    output.push_str("use alloc::borrow::Cow;\n");
    output.push_str("use alloc::boxed::Box;\n");
    output.push_str("use alloc::string::String;\n");
    output.push_str("use alloc::vec;\n");
    output.push_str("use alloc::vec::Vec;\n");
    output.push_str("use core::mem::size_of;\n");
    if schema_uses_maps(schema) {
      output.push_str("use bebop_runtime::HashMap;\n");
    }
    output.push_str("use bebop_runtime::{BebopReader, BebopWriter, BebopEncode, BebopDecode, BebopFlags, DecodeError, Uuid, f16, bf16, BebopTimestamp, BebopDuration};\n");
    output.push_str("use bebop_runtime::wire_size as wire;\n");

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
    analysis: &LifetimeAnalysis,
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
      DefinitionKind::Enum => gen_enum::generate(def, output, &self.options, analysis)?,
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

/// Emit `#[deprecated]` attribute if the definition has a `@deprecated` decorator.
pub fn emit_deprecated(output: &mut String, decorators: &Option<Vec<DecoratorUsage<'_>>>) {
  if let Some(ref decs) = *decorators {
    for dec in decs {
      if dec.fqn.as_deref() == Some("bebop.deprecated") {
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::{borrow::Cow, collections::HashMap};

  use crate::generated::{
    ConstDef, DefinitionDescriptor, DefinitionKind, EnumDef, EnumMemberDescriptor, FieldDescriptor,
    LiteralKind, LiteralValue, MessageDef, SchemaDescriptor, StructDef, TypeDescriptor, TypeKind,
    UnionBranchDescriptor, UnionDef, Visibility,
  };

  fn scalar_type(kind: TypeKind) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(kind),
      ..Default::default()
    }
  }

  fn map_type(
    key: TypeDescriptor<'static>,
    value: TypeDescriptor<'static>,
  ) -> TypeDescriptor<'static> {
    TypeDescriptor {
      kind: Some(TypeKind::Map),
      map_key: Some(Box::new(key)),
      map_value: Some(Box::new(value)),
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

  fn build_trait_schema() -> SchemaDescriptor<'static> {
    let float_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("FloatStruct")),
      fqn: Some(Cow::Borrowed("test.FloatStruct")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
          r#type: Some(scalar_type(TypeKind::Float32)),
          index: Some(0),
          ..Default::default()
        }]),
        ..Default::default()
      }),
      ..Default::default()
    };

    let map_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("MapStruct")),
      fqn: Some(Cow::Borrowed("test.MapStruct")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("entries")),
          r#type: Some(map_type(
            scalar_type(TypeKind::String),
            scalar_type(TypeKind::Uint32),
          )),
          index: Some(0),
          ..Default::default()
        }]),
        ..Default::default()
      }),
      ..Default::default()
    };

    let float_message = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("FloatMessage")),
      fqn: Some(Cow::Borrowed("test.FloatMessage")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
          r#type: Some(scalar_type(TypeKind::Float64)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let float_union = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("FloatUnion")),
      fqn: Some(Cow::Borrowed("test.FloatUnion")),
      union_def: Some(UnionDef {
        branches: Some(vec![UnionBranchDescriptor {
          discriminator: Some(1),
          name: Some(Cow::Borrowed("float_struct")),
          type_ref_fqn: Some(Cow::Borrowed("test.FloatStruct")),
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
        members: Some(vec![EnumMemberDescriptor {
          name: Some(Cow::Borrowed("OK")),
          value: Some(1),
          ..Default::default()
        }]),
        is_flags: Some(false),
      }),
      ..Default::default()
    };

    SchemaDescriptor {
      path: Some(Cow::Borrowed("traits.bop")),
      definitions: Some(vec![
        float_struct,
        map_struct,
        float_message,
        float_union,
        status,
      ]),
      ..Default::default()
    }
  }

  #[test]
  fn emits_file_level_insertion_points() {
    let schema = build_schema();
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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
  fn emits_trait_derives_based_on_float_and_map_content() {
    let schema = build_trait_schema();
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(output.contains("#[derive(Debug, Clone, PartialEq)]\npub struct FloatStruct"));
    assert!(output.contains("#[derive(Debug, Clone, Default, PartialEq)]\npub struct FloatMessage"));
    assert!(output.contains("#[derive(Debug, Clone, PartialEq)]\npub enum FloatUnion<'buf>"));

    // Maps can derive Eq, but not Hash.
    assert!(output.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct MapStruct"));

    // Enums are always hashable since they're integer-backed.
    assert!(output.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Status"));
  }

  #[test]
  fn lifetime_analysis_resolves_forward_references() {
    let a = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("A")),
      fqn: Some(Cow::Borrowed("test.A")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("b")),
          r#type: Some(TypeDescriptor {
            kind: Some(TypeKind::Defined),
            defined_fqn: Some(Cow::Borrowed("test.B")),
            ..Default::default()
          }),
          index: Some(0),
          ..Default::default()
        }]),
        ..Default::default()
      }),
      ..Default::default()
    };

    // B appears after A and is the type that introduces borrowed data.
    let b = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("B")),
      fqn: Some(Cow::Borrowed("test.B")),
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

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("forward.bop")),
      definitions: Some(vec![a, b]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    assert!(analysis.lifetime_fqns.contains("test.B"));
    assert!(analysis.lifetime_fqns.contains("test.A"));

    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");
    assert!(output.contains("pub struct A<'buf>"));
    assert!(output.contains("pub struct B<'buf>"));
  }

  #[test]
  fn parses_visibility_host_option() {
    let mut host_options = HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("crate"));
    let options = GeneratorOptions::from_host_options(Some(&host_options)).unwrap();
    assert_eq!(options.default_visibility, DefaultVisibility::Crate);
  }

  #[test]
  fn rejects_invalid_visibility_host_option() {
    let mut host_options = HashMap::new();
    host_options.insert(Cow::Borrowed("Visibility"), Cow::Borrowed("private"));
    let err = GeneratorOptions::from_host_options(Some(&host_options)).unwrap_err();
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
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::with_options(
      None,
      GeneratorOptions {
        default_visibility: DefaultVisibility::Crate,
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
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("local.bop")),
      definitions: Some(vec![payload, msg, status, flags, result_union]),
      ..Default::default()
    };
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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
      output.contains("  pub(crate) id: Option<i32>,"),
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
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(
      output.contains("pub(crate) const MY_VALUE: i32 = 42i32;"),
      "local const should use pub(crate), got: {}",
      output
    );
  }
}
