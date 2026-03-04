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
        // Forward-compatible unions always need lifetime (Unknown variant uses Cow<'buf, [u8]>).
        // Strict unions only need lifetime if their branches do (resolved in the fixpoint loop).
        if has_decorator(def, "bebop.forward_compatible") {
          analysis.lifetime_fqns.insert(fqn.clone());
        }
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
          Some(DefinitionKind::Union) => {
            // Forward-compatible unions already added above.
            // Strict unions need lifetime only if any branch does.
            if has_decorator(def, "bebop.forward_compatible") {
              true
            } else {
              def
                .union_def
                .as_ref()
                .and_then(|ud| ud.branches.as_deref())
                .is_some_and(|branches| {
                  branches.iter().any(|b| {
                    let branch_fqn = b.type_ref_fqn.as_deref().or(b.inline_fqn.as_deref());
                    branch_fqn.is_some_and(|fqn| analysis.lifetime_fqns.contains(fqn))
                  })
                })
            }
          }
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
    #[derive(Clone, Copy, Default, PartialEq, Eq)]
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

    #[derive(Clone, Default)]
    struct TraitDeps {
      direct: TypeTraits,
      deps: Vec<String>,
    }

    impl TraitDeps {
      fn combine(&mut self, other: Self) {
        self.direct = self.direct.combine(other.direct);
        for dep in other.deps {
          self.add_dep(dep);
        }
      }

      fn add_dep(&mut self, dep: String) {
        if !self.deps.iter().any(|d| d == &dep) {
          self.deps.push(dep);
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

    fn type_deps(td: &TypeDescriptor) -> TraitDeps {
      let kind = match td.kind {
        Some(kind) => kind,
        None => return TraitDeps::default(),
      };

      match kind {
        TypeKind::Float16 | TypeKind::Float32 | TypeKind::Float64 | TypeKind::Bfloat16 => {
          TraitDeps {
            direct: TypeTraits {
              has_float: true,
              has_map: false,
            },
            deps: Vec::new(),
          }
        }
        TypeKind::Array => td
          .array_element
          .as_deref()
          .map(type_deps)
          .unwrap_or_default(),
        TypeKind::FixedArray => td
          .fixed_array_element
          .as_deref()
          .map(type_deps)
          .unwrap_or_default(),
        TypeKind::Map => {
          let mut deps = TraitDeps {
            direct: TypeTraits {
              has_float: false,
              has_map: true,
            },
            deps: Vec::new(),
          };
          if let Some(key) = td.map_key.as_deref() {
            deps.combine(type_deps(key));
          }
          if let Some(value) = td.map_value.as_deref() {
            deps.combine(type_deps(value));
          }
          deps
        }
        TypeKind::Defined => {
          let mut deps = TraitDeps::default();
          if let Some(fqn) = td.defined_fqn.as_deref() {
            deps.add_dep(fqn.to_string());
          } else {
            deps.direct = TypeTraits {
              has_float: true,
              has_map: true,
            };
          }
          deps
        }
        _ => TraitDeps::default(),
      }
    }

    fn field_list_deps(fields: Option<&[FieldDescriptor]>) -> TraitDeps {
      fields
        .unwrap_or(&[])
        .iter()
        .fold(TraitDeps::default(), |mut acc, field| {
          if let Some(td) = field.r#type.as_ref() {
            acc.combine(type_deps(td));
          }
          acc
        })
    }

    fn definition_deps(def: &DefinitionDescriptor) -> TraitDeps {
      match def.kind {
        Some(DefinitionKind::Enum) => TraitDeps::default(),
        Some(DefinitionKind::Struct) => {
          field_list_deps(def.struct_def.as_ref().and_then(|sd| sd.fields.as_deref()))
        }
        Some(DefinitionKind::Message) => {
          field_list_deps(def.message_def.as_ref().and_then(|md| md.fields.as_deref()))
        }
        Some(DefinitionKind::Union) => {
          let mut deps = TraitDeps::default();
          for branch in def
            .union_def
            .as_ref()
            .and_then(|ud| ud.branches.as_deref())
            .unwrap_or(&[])
          {
            if let Some(branch_fqn) = branch
              .type_ref_fqn
              .as_deref()
              .or(branch.inline_fqn.as_deref())
            {
              deps.add_dep(branch_fqn.to_string());
            } else if let (Some(union_fqn), Some(branch_name)) =
              (def.fqn.as_deref(), branch.name.as_deref())
            {
              deps.add_dep(format!("{}.{}", union_fqn, naming::type_name(branch_name)));
            } else {
              deps.direct = deps.direct.combine(TypeTraits {
                has_float: true,
                has_map: true,
              });
            }
          }
          deps
        }
        _ => TraitDeps::default(),
      }
    }

    let mut def_by_fqn: HashMap<String, &DefinitionDescriptor> = HashMap::new();
    for schema in schemas {
      let defs = schema.definitions.as_deref().unwrap_or(&[]);
      collect_definitions(defs, &mut def_by_fqn);
    }

    let mut deps_by_fqn: HashMap<String, (DefinitionKind, TraitDeps)> = HashMap::new();
    for (fqn, def) in &def_by_fqn {
      if let Some(kind) = def.kind {
        if kind_supports_derives(kind) {
          deps_by_fqn.insert(fqn.clone(), (kind, definition_deps(def)));
        }
      }
    }

    let mut resolved: HashMap<String, TypeTraits> = deps_by_fqn
      .iter()
      .map(|(fqn, (_, deps))| (fqn.clone(), deps.direct))
      .collect();

    let mut changed = true;
    while changed {
      changed = false;
      for (fqn, (_, deps)) in &deps_by_fqn {
        let mut traits = deps.direct;
        for dep in &deps.deps {
          if let Some(dep_traits) = resolved.get(dep) {
            traits = traits.combine(*dep_traits);
          } else {
            traits = traits.combine(TypeTraits {
              has_float: true,
              has_map: true,
            });
          }
        }
        if resolved.get(fqn).copied() != Some(traits) {
          resolved.insert(fqn.clone(), traits);
          changed = true;
        }
      }
    }

    for (fqn, (kind, _)) in &deps_by_fqn {
      if *kind == DefinitionKind::Enum {
        self.eq_fqns.insert(fqn.clone());
        self.hash_fqns.insert(fqn.clone());
        continue;
      }

      let traits = resolved.get(fqn).copied().unwrap_or_default();
      if !traits.has_float {
        self.eq_fqns.insert(fqn.clone());
      }
      if !traits.has_float && !traits.has_map {
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
    output.push_str("#![allow(warnings)]\n");
    output.push_str("#![no_implicit_prelude]\n\n");
    output.push_str("extern crate alloc;\n");
    output.push_str("extern crate bebop_runtime;\n");
    output.push_str("extern crate core;\n");
    output.push_str("use core::convert::Into as _;\n");
    output.push_str("use core::iter::IntoIterator as _;\n");
    output.push_str("use core::iter::Iterator as _;\n");
    output.push_str("use alloc::vec;\n");
    output.push_str("use bebop_runtime::{BebopDecode, BebopDuration, BebopEncode, BebopFlags, BebopReader, BebopTimestamp, BebopWriter, DecodeError, bf16, f16};\n");
    output.push_str("use bebop_runtime::wire_size as wire;\n");
    output.push_str("#[cfg(feature = \"serde\")]\nuse bebop_runtime::serde;\n");

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

/// Returns `true` if the definition has a decorator matching `name`.
/// Matches both bare names (`"forward_compatible"`) and fully-qualified
/// names (`"bebop.forward_compatible"`).
pub fn has_decorator(def: &DefinitionDescriptor, name: &str) -> bool {
  def.decorators.as_ref().is_some_and(|decs| {
    decs.iter().any(|d| {
      d.fqn
        .as_deref()
        .is_some_and(|fqn| fqn == name || name.ends_with(&format!(".{}", fqn)))
    })
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{borrow::Cow, collections::HashMap};

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
      decorators: Some(forward_compatible_decorator()),
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

  fn build_recursive_union_schema() -> SchemaDescriptor<'static> {
    let json_null = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("JsonNull")),
      fqn: Some(Cow::Borrowed("JsonNull")),
      struct_def: Some(StructDef {
        fields: Some(vec![]),
        fixed_size: Some(0),
        ..Default::default()
      }),
      ..Default::default()
    };

    let bool_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Bool")),
      fqn: Some(Cow::Borrowed("JsonValue.Bool")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
          r#type: Some(scalar_type(TypeKind::Bool)),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let number_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Number")),
      fqn: Some(Cow::Borrowed("JsonValue.Number")),
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

    let list_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("List")),
      fqn: Some(Cow::Borrowed("JsonValue.List")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("values")),
          r#type: Some(array_type(defined_type("JsonValue"))),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let object_msg = DefinitionDescriptor {
      kind: Some(DefinitionKind::Message),
      name: Some(Cow::Borrowed("Object")),
      fqn: Some(Cow::Borrowed("JsonValue.Object")),
      message_def: Some(MessageDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("fields")),
          r#type: Some(map_type(
            scalar_type(TypeKind::String),
            defined_type("JsonValue"),
          )),
          index: Some(1),
          ..Default::default()
        }]),
      }),
      ..Default::default()
    };

    let json_value = DefinitionDescriptor {
      kind: Some(DefinitionKind::Union),
      name: Some(Cow::Borrowed("JsonValue")),
      fqn: Some(Cow::Borrowed("JsonValue")),
      union_def: Some(UnionDef {
        branches: Some(vec![
          UnionBranchDescriptor {
            discriminator: Some(1),
            name: Some(Cow::Borrowed("null")),
            type_ref_fqn: Some(Cow::Borrowed("JsonNull")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(2),
            name: Some(Cow::Borrowed("bool")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Bool")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(3),
            name: Some(Cow::Borrowed("number")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Number")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(4),
            name: Some(Cow::Borrowed("list")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.List")),
            ..Default::default()
          },
          UnionBranchDescriptor {
            discriminator: Some(5),
            name: Some(Cow::Borrowed("object")),
            type_ref_fqn: Some(Cow::Borrowed("JsonValue.Object")),
            ..Default::default()
          },
        ]),
      }),
      nested: Some(vec![bool_msg, number_msg, list_msg, object_msg]),
      decorators: Some(forward_compatible_decorator()),
      ..Default::default()
    };

    SchemaDescriptor {
      path: Some(Cow::Borrowed("recursive-union.bop")),
      definitions: Some(vec![json_null, json_value]),
      ..Default::default()
    }
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
  fn recursive_union_float_propagates_into_object_eq_derives() {
    let schema = build_recursive_union_schema();
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(output.contains("#[derive(Debug, Clone, PartialEq)]\npub enum JsonValue<'buf>"));
    assert!(output.contains("#[derive(Debug, Clone, Default, PartialEq)]\npub struct Object<'buf>"));
  }

  #[test]
  fn struct_constructor_converts_owned_string_arrays() {
    let schema = build_string_array_constructor_schema();
    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    assert!(output.contains("tokens: alloc::vec::Vec<alloc::string::String>"));
    assert!(
      output.contains(
        "let tokens = tokens.into_iter().map(|_e| alloc::borrow::Cow::Owned(_e)).collect();"
      ),
      "expected constructor conversion for Vec<String> -> Vec<Cow<str>>; output:\n{}",
      output
    );
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
      decorators: Some(forward_compatible_decorator()),
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
      output.contains("  pub(crate) id: ::core::option::Option<i32>,"),
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

  fn forward_compatible_decorator() -> Vec<DecoratorUsage<'static>> {
    vec![DecoratorUsage {
      fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
      ..Default::default()
    }]
  }

  #[test]
  fn strict_union_omits_lifetime_when_branches_are_scalar() {
    // Union with only scalar branches and no @forward_compatible → no 'buf
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Inner")),
      fqn: Some(Cow::Borrowed("test.Inner")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
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
          name: Some(Cow::Borrowed("inner")),
          type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
          ..Default::default()
        }]),
      }),
      // No decorators — strict mode
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("strict.bop")),
      definitions: Some(vec![inner_struct, strict_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    assert!(
      !analysis.lifetime_fqns.contains("test.StrictUnion"),
      "strict union with scalar-only branches should NOT need lifetime"
    );
  }

  #[test]
  fn forward_compatible_union_always_has_lifetime() {
    let inner_struct = DefinitionDescriptor {
      kind: Some(DefinitionKind::Struct),
      name: Some(Cow::Borrowed("Inner")),
      fqn: Some(Cow::Borrowed("test.Inner")),
      struct_def: Some(StructDef {
        fields: Some(vec![FieldDescriptor {
          name: Some(Cow::Borrowed("value")),
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
          name: Some(Cow::Borrowed("inner")),
          type_ref_fqn: Some(Cow::Borrowed("test.Inner")),
          ..Default::default()
        }]),
      }),
      decorators: Some(vec![DecoratorUsage {
        fqn: Some(Cow::Borrowed("bebop.forward_compatible")),
        ..Default::default()
      }]),
      ..Default::default()
    };

    let schema = SchemaDescriptor {
      path: Some(Cow::Borrowed("fc.bop")),
      definitions: Some(vec![inner_struct, fc_union]),
      ..Default::default()
    };

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    assert!(
      analysis.lifetime_fqns.contains("test.FcUnion"),
      "forward-compatible union should always need lifetime"
    );
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

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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
    assert!(output.contains("impl ::core::convert::From<u32> for Color"));
    assert!(!output.contains("TryFrom"));
    // Should use discriminator() in encode
    assert!(output.contains("self.discriminator()"));
    // Decode should use From
    assert!(output.contains("Self::from(value)"));
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

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have Unknown variant with Cow
    assert!(output.contains("Unknown(u8, alloc::borrow::Cow<'buf, [u8]>)"));
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

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
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

    let analysis = LifetimeAnalysis::build_all(std::slice::from_ref(&schema));
    let output = RustGenerator::new(None)
      .generate(&schema, &[], &analysis)
      .expect("generator should succeed");

    // Should have reader.skip (forward-compatible behavior)
    assert!(output.contains("reader.skip("));
    // Should NOT have InvalidField
    assert!(!output.contains("InvalidField"));
  }
}
