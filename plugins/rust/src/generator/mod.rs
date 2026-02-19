pub mod gen_const;
pub mod gen_enum;
pub mod gen_message;
pub mod gen_service;
pub mod gen_struct;
pub mod gen_union;
pub mod naming;
pub mod type_mapper;

use std::borrow::Cow;
use std::collections::HashSet;

use crate::error::GeneratorError;
use crate::generated::*;

/// Pre-computed lifetime and kind information for all definitions in a schema.
pub struct LifetimeAnalysis {
  /// FQNs of enum definitions (never need a lifetime parameter).
  pub enum_fqns: HashSet<String>,
  /// FQNs of types that need `'buf` (contain strings, byte arrays, or unions).
  pub lifetime_fqns: HashSet<String>,
}

impl LifetimeAnalysis {
  /// Build a combined analysis from all schemas, so cross-schema type
  /// references resolve correctly.
  pub fn build_all(schemas: &[SchemaDescriptor]) -> Self {
    let mut analysis = LifetimeAnalysis {
      enum_fqns: HashSet::new(),
      lifetime_fqns: HashSet::new(),
    };
    for schema in schemas {
      let definitions = schema.definitions.as_deref().unwrap_or(&[]);
      for def in definitions {
        analysis.analyze_def(def);
      }
    }
    analysis
  }

  fn analyze_def(&mut self, def: &DefinitionDescriptor) {
    let kind = match def.kind {
      Some(k) => k,
      None => return,
    };
    let fqn = match def.fqn.as_deref() {
      Some(f) => f.to_string(),
      None => return,
    };

    match kind {
      DefinitionKind::Enum => {
        self.enum_fqns.insert(fqn);
      }
      DefinitionKind::Struct => {
        if let Some(ref sd) = def.struct_def {
          let fields = sd.fields.as_deref().unwrap_or(&[]);
          if fields.iter().any(|f| {
            f.r#type
              .as_ref()
              .is_some_and(|td| self.type_needs_lifetime(td))
          }) {
            self.lifetime_fqns.insert(fqn);
          }
        }
      }
      DefinitionKind::Message => {
        if let Some(ref md) = def.message_def {
          let fields = md.fields.as_deref().unwrap_or(&[]);
          if fields.iter().any(|f| {
            f.r#type
              .as_ref()
              .is_some_and(|td| self.type_needs_lifetime(td))
          }) {
            self.lifetime_fqns.insert(fqn);
          }
        }
      }
      DefinitionKind::Union => {
        // Unions always need lifetime (Unknown variant uses Cow<'buf, [u8]>)
        self.lifetime_fqns.insert(fqn);
      }
      _ => {}
    }

    // Process nested definitions
    if let Some(ref nested) = def.nested {
      for child in nested {
        self.analyze_def(child);
      }
    }
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
}

impl RustGenerator {
  pub fn new(compiler_version: Option<VersionOwned>) -> Self {
    Self { compiler_version }
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
    output.push_str("use bebop_runtime::{BebopReader, BebopWriter, BebopEncode, BebopDecode, BebopFlags, DecodeError, f16, bf16};\n");
    output.push_str("use bebop_runtime::wire_size as wire;\n");

    // Cross-module imports for sibling schemas
    for stem in sibling_imports {
      output.push_str(&format!("use super::{}::*;\n", stem));
    }
    output.push('\n');

    for def in definitions {
      Self::generate_definition(def, &mut output, 0, analysis)?;
    }

    Ok(output)
  }

  fn generate_definition(
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
      DefinitionKind::Enum => gen_enum::generate(def, output, analysis)?,
      DefinitionKind::Struct => gen_struct::generate(def, output, analysis)?,
      DefinitionKind::Message => gen_message::generate(def, output, analysis)?,
      DefinitionKind::Union => gen_union::generate(def, output, analysis)?,
      DefinitionKind::Const => gen_const::generate(def, output)?,
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
        Self::generate_definition(child, output, depth + 1, analysis)?;
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
