use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

use super::naming::{field_name, type_name};
use super::type_mapper;
use super::{emit_deprecated, emit_doc_comment, LifetimeAnalysis};

/// Generate Rust code for a struct definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let fqn = def.fqn.as_deref().unwrap_or("");

  let struct_def = def
    .struct_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("struct missing struct_def".into()))?;

  let fields = struct_def.fields.as_deref().unwrap_or(&[]);
  let has_lifetime = analysis.lifetime_fqns.contains(fqn);

  let lt = if has_lifetime { "<'buf>" } else { "" };
  let lt_wild = if has_lifetime { "<'_>" } else { "" };

  // Collect field info upfront
  let fnames: Vec<String> = fields
    .iter()
    .map(|f| field_name(f.name.as_deref().unwrap_or("unknown")))
    .collect();

  let cow_types: Vec<String> = fields
    .iter()
    .map(|f| {
      let td = f.r#type.as_ref().ok_or_else(|| {
        GeneratorError::MalformedDefinition("struct field missing type".into())
      })?;
      type_mapper::rust_type_cow(td, analysis)
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  let owned_types: Vec<String> = fields
    .iter()
    .map(|f| {
      let td = f.r#type.as_ref().ok_or_else(|| {
        GeneratorError::MalformedDefinition("struct field missing type".into())
      })?;
      type_mapper::rust_type_owned(td, analysis)
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  // ── Doc comment + deprecated ──────────────────────────────────
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // ── Derive + struct definition ────────────────────────────────
  output.push_str("#[derive(Debug, Clone)]\n");
  output.push_str(&format!("pub struct {}{} {{\n", name, lt));
  for (i, f) in fields.iter().enumerate() {
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    output.push_str(&format!("  pub {}: {},\n", fnames[i], cow_types[i]));
  }
  output.push_str("}\n\n");

  // ── Type alias ────────────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!(
      "pub type {}Owned = {}<'static>;\n\n",
      name, name
    ));
  } else {
    output.push_str(&format!("pub type {}Owned = {};\n\n", name, name));
  }

  // ── new() constructor ─────────────────────────────────────────
  output.push_str(&format!("impl {}{} {{\n", name, lt_wild));
  output.push_str("  pub fn new(");
  for (i, fname) in fnames.iter().enumerate() {
    if i > 0 {
      output.push_str(", ");
    }
    output.push_str(&format!("{}: {}", fname, owned_types[i]));
  }
  output.push_str(") -> Self {\n");
  output.push_str("    Self {\n");
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().unwrap();
    if type_mapper::is_cow_field(td) {
      // String → Cow::Owned(v), Vec<u8> → Cow::Owned(v)
      output.push_str(&format!(
        "      {}: Cow::Owned({}),\n",
        fnames[i], fnames[i]
      ));
    } else {
      output.push_str(&format!("      {},\n", fnames[i]));
    }
  }
  output.push_str("    }\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  // ── into_owned() (only when has_lifetime) ─────────────────────
  if has_lifetime {
    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
    output.push_str(&format!(
      "  pub fn into_owned(self) -> {}<'static> {{\n",
      name
    ));
    output.push_str(&format!("    {} {{\n", name));
    for (i, f) in fields.iter().enumerate() {
      let td = f.r#type.as_ref().unwrap();
      let expr =
        type_mapper::into_owned_expression(td, &format!("self.{}", fnames[i]), analysis)?;
      output.push_str(&format!("      {}: {},\n", fnames[i], expr));
    }
    output.push_str("    }\n");
    output.push_str("  }\n");
    output.push_str("}\n\n");
  }

  // ── impl BebopEncode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl{} BebopEncode for {}{} {{\n",
    lt, name, lt
  ));

  // FIXED_ENCODED_SIZE
  if let Some(fs) = struct_def.fixed_size {
    if fs > 0 {
      output.push_str(&format!(
        "  const FIXED_ENCODED_SIZE: Option<usize> = Some({});\n\n",
        fs
      ));
    }
  }

  // encode()
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().ok_or_else(|| {
      GeneratorError::MalformedDefinition("struct field missing type".into())
    })?;
    let write_stmt =
      type_mapper::write_expression_cow(td, &format!("self.{}", fnames[i]), "writer", analysis)?;
    output.push_str(&format!("    {};\n", write_stmt));
  }
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  if let Some(fs) = struct_def.fixed_size {
    if fs > 0 {
      output.push_str(&format!("    {}\n", fs));
    } else {
      emit_encoded_size_body(fields, &fnames, output, analysis)?;
    }
  } else {
    emit_encoded_size_body(fields, &fnames, output, analysis)?;
  }
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {}{} {{\n",
    name, lt
  ));
  output.push_str(
    "  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {\n",
  );
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().ok_or_else(|| {
      GeneratorError::MalformedDefinition("struct field missing type".into())
    })?;
    let read_expr = type_mapper::read_expression_cow(td, "reader", analysis)?;
    output.push_str(&format!("    let {} = {}?;\n", fnames[i], read_expr));
  }
  output.push_str(&format!("    Ok({} {{\n", name));
  for fname in &fnames {
    output.push_str(&format!("      {},\n", fname));
  }
  output.push_str("    })\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  Ok(())
}

/// Emit the body of `encoded_size()` for variable-size structs.
fn emit_encoded_size_body(
  fields: &[crate::generated::FieldDescriptor],
  fnames: &[String],
  output: &mut String,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let mut parts: Vec<String> = Vec::new();
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().ok_or_else(|| {
      GeneratorError::MalformedDefinition("struct field missing type".into())
    })?;
    let expr =
      type_mapper::encoded_size_expression(td, &format!("self.{}", fnames[i]), analysis)?;
    parts.push(expr);
  }
  if parts.is_empty() {
    output.push_str("    0\n");
  } else {
    output.push_str(&format!("    {}\n", parts.join(" + ")));
  }
  Ok(())
}
