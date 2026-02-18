use crate::descriptor::DefinitionDescriptor;
use crate::error::GeneratorError;

use super::naming::{field_name, type_name};
use super::type_mapper;
use super::{emit_deprecated, emit_doc_comment};

/// Generate Rust code for a struct definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));

  let struct_def = def
    .struct_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("struct missing struct_def".into()))?;

  let fields = struct_def.fields.as_deref().unwrap_or(&[]);

  // Collect field info upfront
  let field_info: Vec<(String, String)> = fields
    .iter()
    .map(|f| {
      let fname = field_name(f.name.as_deref().unwrap_or("unknown"));
      let ftype = f
        .field_type
        .as_ref()
        .map(|td| type_mapper::rust_type(td))
        .transpose()?
        .unwrap_or_else(|| "()".to_string());
      Ok((fname, ftype))
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Derive + struct definition
  output.push_str("#[derive(Debug, Clone)]\n");
  output.push_str(&format!("pub struct {} {{\n", name));
  for (i, f) in fields.iter().enumerate() {
    let (ref fname, ref ftype) = field_info[i];
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    output.push_str(&format!("  pub {}: {},\n", fname, ftype));
  }
  output.push_str("}\n\n");

  // impl decode + encode
  output.push_str(&format!("impl {} {{\n", name));

  // decode
  output.push_str("  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {\n");
  for (i, f) in fields.iter().enumerate() {
    let (ref fname, _) = field_info[i];
    let td = f
      .field_type
      .as_ref()
      .ok_or_else(|| GeneratorError::MalformedDefinition("struct field missing type".into()))?;
    let read_expr = type_mapper::read_expression(td, "reader")?;
    output.push_str(&format!("    let {} = {}?;\n", fname, read_expr));
  }
  output.push_str(&format!("    Ok({} {{\n", name));
  for (ref fname, _) in &field_info {
    output.push_str(&format!("      {},\n", fname));
  }
  output.push_str("    })\n");
  output.push_str("  }\n\n");

  // encode
  output.push_str("  pub fn encode(&self, writer: &mut BebopWriter) {\n");
  for (i, f) in fields.iter().enumerate() {
    let (ref fname, _) = field_info[i];
    let td = f
      .field_type
      .as_ref()
      .ok_or_else(|| GeneratorError::MalformedDefinition("struct field missing type".into()))?;
    let write_stmt = type_mapper::write_expression(td, &format!("self.{}", fname), "writer")?;
    output.push_str(&format!("    {};\n", write_stmt));
  }
  output.push_str("  }\n");

  output.push_str("}\n\n");

  Ok(())
}
