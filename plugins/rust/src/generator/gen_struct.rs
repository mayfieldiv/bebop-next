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
  let impl_header = if has_lifetime {
    format!("impl<'buf> {}<'buf> {{\n", name)
  } else {
    format!("impl {} {{\n", name)
  };

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
  }

  // ── new() constructor + FIXED_ENCODED_SIZE ──────────────────────
  output.push_str(&impl_header);
  if let Some(fs) = struct_def.fixed_size {
    if fs > 0 {
      let mut parts: Vec<String> = Vec::with_capacity(fields.len());
      let mut can_emit_expr = true;
      for f in fields {
        let td = f.r#type.as_ref().ok_or_else(|| {
          GeneratorError::MalformedDefinition("struct field missing type".into())
        })?;
        match type_mapper::fixed_encoded_size_expression(td)? {
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
  for (i, fname) in fnames.iter().enumerate() {
    if i > 0 {
      output.push_str(", ");
    }
    let td = fields[i].r#type.as_ref().ok_or_else(|| {
      GeneratorError::MalformedDefinition("struct field missing type".into())
    })?;
    if has_lifetime && type_mapper::is_cow_field(td) {
      output.push_str(&format!("{}: impl Into<{}>", fname, cow_types[i]));
    } else {
      output.push_str(&format!("{}: {}", fname, owned_types[i]));
    }
  }
  output.push_str(") -> Self {\n");
  let mut init_fields: Vec<String> = Vec::with_capacity(fields.len());
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().unwrap();
    if has_lifetime && type_mapper::is_cow_field(td) {
      output.push_str(&format!("    let {} = {}.into();\n", fnames[i], fnames[i]));
      init_fields.push(fnames[i].clone());
    } else if type_mapper::is_cow_field(td) {
      // Defensive fallback for non-lifetime cases.
      output.push_str(&format!(
        "    let {} = Cow::Owned({});\n",
        fnames[i], fnames[i]
      ));
      init_fields.push(fnames[i].clone());
    } else {
      init_fields.push(fnames[i].clone());
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
    output.push_str(&format!(
      "  pub fn into_owned(self) -> {}Owned {{\n",
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
      output.push_str("    Self::FIXED_ENCODED_SIZE\n");
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
    if let Some(read_expr) = type_mapper::borrowed_cow_read_expression(td, "reader") {
      output.push_str(&format!("    let {} = {};\n", fnames[i], read_expr));
    } else {
      let read_expr = type_mapper::read_expression_cow(td, "reader", analysis)?;
      output.push_str(&format!("    let {} = {}?;\n", fnames[i], read_expr));
    }
  }
  if fnames.is_empty() {
    output.push_str(&format!("    Ok({} {{}})\n", name));
  } else {
    output.push_str(&format!("    Ok({} {{ {} }})\n", name, fnames.join(", ")));
  }
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
  output.push_str("    let mut size = 0;\n");
  for (i, f) in fields.iter().enumerate() {
    let td = f.r#type.as_ref().ok_or_else(|| {
      GeneratorError::MalformedDefinition("struct field missing type".into())
    })?;
    let expr =
      type_mapper::encoded_size_expression(td, &format!("self.{}", fnames[i]), analysis)?;
    output.push_str(&format!("    size += {};\n", expr));
  }
  output.push_str("    size\n");
  Ok(())
}
