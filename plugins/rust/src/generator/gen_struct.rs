use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeDescriptor};

use super::naming::{field_name, type_name};
use super::type_mapper;
use super::{
  emit_deprecated, emit_doc_comment, visibility_keyword, GeneratorOptions, LifetimeAnalysis,
};

struct StructFieldMeta<'a> {
  fname: String,
  cow_type: String,
  owned_type: String,
  td: &'a TypeDescriptor<'a>,
}

/// Generate Rust code for a struct definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  options: &GeneratorOptions,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let fqn = def.fqn.as_deref().unwrap_or("");

  let struct_def = def
    .struct_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("struct missing struct_def".into()))?;

  let fields = struct_def.fields.as_deref().unwrap_or(&[]);
  let vis = visibility_keyword(def, options);
  let has_lifetime = analysis.lifetime_fqns.contains(fqn);

  let lt = if has_lifetime { "<'buf>" } else { "" };
  let impl_header = if has_lifetime {
    format!("impl<'buf> {}<'buf> {{\n", name)
  } else {
    format!("impl {} {{\n", name)
  };

  // Collect field metadata once and reuse it through generation.
  let field_metas: Vec<StructFieldMeta<'_>> = fields
    .iter()
    .map(|f| {
      let td = f
        .r#type
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedDefinition("struct field missing type".into()))?;
      Ok(StructFieldMeta {
        fname: field_name(f.name.as_deref().unwrap_or("unknown")),
        cow_type: type_mapper::rust_type(td, analysis)?,
        owned_type: type_mapper::rust_type_owned(td, analysis)?,
        td,
      })
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  // ── Doc comment + deprecated ──────────────────────────────────
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // ── Derive + struct definition ────────────────────────────────
  let mut derives = vec!["Debug", "Clone", "PartialEq"];
  if analysis.can_derive_eq(fqn) {
    derives.push("Eq");
  }
  if analysis.can_derive_hash(fqn) {
    derives.push("Hash");
  }
  output
    .push_str("#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]\n");
  output.push_str(&format!("#[derive({})]\n", derives.join(", ")));
  output.push_str(&format!("{} struct {}{} {{\n", vis, name, lt));
  for (f, meta) in fields.iter().zip(&field_metas) {
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    if type_mapper::is_byte_array_cow_field(meta.td) {
      output.push_str("  #[cfg_attr(feature = \"serde\", serde(borrow))]\n");
      output.push_str(
        "  #[cfg_attr(feature = \"serde\", serde(with = \"bebop_runtime::serde_cow_bytes\"))]\n",
      );
    }
    output.push_str(&format!("  {} {}: {},\n", vis, meta.fname, meta.cow_type));
  }
  output.push_str("}\n\n");

  // ── Type alias ────────────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!(
      "{} type {}Owned = {}<'static>;\n\n",
      vis, name, name
    ));
  }

  // ── new() constructor + FIXED_ENCODED_SIZE ──────────────────────
  output.push_str(&impl_header);
  if let Some(fs) = struct_def.fixed_size {
    if fs > 0 {
      let mut parts: Vec<String> = Vec::with_capacity(field_metas.len());
      let mut can_emit_expr = true;
      for meta in &field_metas {
        match type_mapper::fixed_encoded_size_expression(meta.td)? {
          Some(expr) => parts.push(expr),
          None => {
            can_emit_expr = false;
            break;
          }
        }
      }
      if can_emit_expr && !parts.is_empty() {
        if parts.len() > 2 {
          output.push_str("  pub const FIXED_ENCODED_SIZE: usize =\n");
          output.push_str(&format!("    {}\n", parts[0]));
          for (i, expr) in parts.iter().enumerate().skip(1) {
            if i == parts.len() - 1 {
              output.push_str(&format!("    + {};\n\n", expr));
            } else {
              output.push_str(&format!("    + {}\n", expr));
            }
          }
        } else {
          output.push_str(&format!(
            "  pub const FIXED_ENCODED_SIZE: usize = {};\n\n",
            parts.join(" + ")
          ));
        }
      } else {
        output.push_str(&format!(
          "  pub const FIXED_ENCODED_SIZE: usize = {};\n\n",
          fs
        ));
      }
    }
  }
  output.push_str("  pub fn new(");
  for (i, meta) in field_metas.iter().enumerate() {
    if i > 0 {
      output.push_str(", ");
    }
    if has_lifetime && type_mapper::is_cow_field(meta.td) {
      output.push_str(&format!("{}: impl Into<{}>", meta.fname, meta.cow_type));
    } else {
      output.push_str(&format!("{}: {}", meta.fname, meta.owned_type));
    }
  }
  output.push_str(") -> Self {\n");
  let mut init_fields: Vec<String> = Vec::with_capacity(field_metas.len());
  for meta in &field_metas {
    if has_lifetime && type_mapper::is_cow_field(meta.td) {
      output.push_str(&format!(
        "    let {} = {}.into();\n",
        meta.fname, meta.fname
      ));
      init_fields.push(meta.fname.clone());
    } else if type_mapper::is_cow_field(meta.td) {
      // Defensive fallback for non-lifetime cases.
      output.push_str(&format!(
        "    let {} = Cow::Owned({});\n",
        meta.fname, meta.fname
      ));
      init_fields.push(meta.fname.clone());
    } else {
      init_fields.push(meta.fname.clone());
    }
  }
  if init_fields.is_empty() {
    output.push_str("    Self {}\n");
  } else {
    output.push_str(&format!("    Self {{ {} }}\n", init_fields.join(", ")));
  }
  output.push_str("  }\n");
  output.push_str("}\n\n");

  // ── into_owned() (only when has_lifetime) ─────────────────────
  if has_lifetime {
    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
    output.push_str(&format!("  pub fn into_owned(self) -> {}Owned {{\n", name));
    output.push_str(&format!("    {} {{\n", name));
    for meta in &field_metas {
      let expr =
        type_mapper::into_owned_expression(meta.td, &format!("self.{}", meta.fname), analysis)?;
      output.push_str(&format!("      {}: {},\n", meta.fname, expr));
    }
    output.push_str("    }\n");
    output.push_str("  }\n");
    output.push_str("}\n\n");
  }

  // ── impl BebopEncode ──────────────────────────────────────────
  output.push_str(&format!("impl{} BebopEncode for {}{} {{\n", lt, name, lt));

  // encode()
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    name
  ));
  for meta in &field_metas {
    let write_stmt =
      type_mapper::write_expression(meta.td, &format!("self.{}", meta.fname), "writer", analysis)?;
    output.push_str(&format!("    {};\n", write_stmt));
  }
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  if let Some(fs) = struct_def.fixed_size {
    if fs > 0 {
      output.push_str("    Self::FIXED_ENCODED_SIZE\n");
    } else {
      emit_encoded_size_body(&field_metas, output, analysis)?;
    }
  } else {
    emit_encoded_size_body(&field_metas, output, analysis)?;
  }
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {}{} {{\n",
    name, lt
  ));
  output.push_str("  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  for meta in &field_metas {
    if let Some(read_expr) = type_mapper::borrowed_cow_read_expression(meta.td, "reader") {
      output.push_str(&format!("    let {} = {};\n", meta.fname, read_expr));
    } else {
      let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
      output.push_str(&format!("    let {} = {}?;\n", meta.fname, read_expr));
    }
  }
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  if field_metas.is_empty() {
    output.push_str(&format!("    Ok({} {{}})\n", name));
  } else {
    let init_fields = field_metas
      .iter()
      .map(|meta| meta.fname.as_str())
      .collect::<Vec<_>>()
      .join(", ");
    output.push_str(&format!("    Ok({} {{ {} }})\n", name, init_fields));
  }
  output.push_str("  }\n");
  output.push_str("}\n\n");

  output.push_str(&format!("impl{} {}{} {{\n", lt, name, lt));
  output.push_str(&format!(
    "  // @@bebop_insertion_point(struct_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  Ok(())
}

/// Emit the body of `encoded_size()` for variable-size structs.
fn emit_encoded_size_body(
  field_metas: &[StructFieldMeta<'_>],
  output: &mut String,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  output.push_str("    let mut size = 0;\n");
  for meta in field_metas {
    let expr =
      type_mapper::encoded_size_expression(meta.td, &format!("self.{}", meta.fname), analysis)?;
    output.push_str(&format!("    size += {};\n", expr));
  }
  output.push_str("    size\n");
  Ok(())
}
