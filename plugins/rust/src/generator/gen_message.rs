use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeKind};

use super::naming::{field_name, type_name};
use super::type_mapper;
use super::{emit_deprecated, emit_doc_comment};

/// Wrapping strategy for a message field.
enum FieldWrap {
  /// `Option<T>` — normal message field
  Plain,
  /// `Option<Box<T>>` — self-referential field (direct DEFINED reference to own type)
  Boxed,
  /// `Option<Vec<T>>` — array of self (Vec provides indirection)
  VecIndirect,
}

/// Determine wrapping for a message field.
fn field_wrap(field_type: &crate::generated::TypeDescriptor, own_fqn: &str) -> FieldWrap {
  let kind = field_type.kind.unwrap_or(TypeKind::Unknown);

  // Direct self-reference: field's DEFINED fqn == own fqn
  if kind == TypeKind::Defined {
    if let Some(ref fqn) = field_type.defined_fqn {
      if fqn == own_fqn {
        return FieldWrap::Boxed;
      }
    }
  }

  // Array of self: element is DEFINED with fqn == own fqn — Vec provides indirection
  if kind == TypeKind::Array {
    if let Some(ref elem) = field_type.array_element {
      if elem.kind == Some(TypeKind::Defined) {
        if let Some(ref fqn) = elem.defined_fqn {
          if fqn == own_fqn {
            return FieldWrap::VecIndirect;
          }
        }
      }
    }
  }

  FieldWrap::Plain
}

/// Returns true if a scalar TypeKind value should be dereferenced when writing from `&T`.
fn scalar_needs_deref(kind: TypeKind) -> bool {
  // String and UUID are passed by reference, all other scalars are Copy and need *v
  kind != TypeKind::String && kind != TypeKind::Uuid
}

/// Generate Rust code for a message definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let own_fqn = def.fqn.as_deref().unwrap_or("");

  let message_def = def
    .message_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("message missing message_def".into()))?;

  let fields = message_def.fields.as_deref().unwrap_or(&[]);

  // Pre-compute field metadata
  struct FieldMeta {
    fname: String,
    rust_type: String,
    tag: u32,
    wrap: FieldWrap,
  }

  let field_metas: Vec<FieldMeta> = fields
    .iter()
    .map(|f| {
      let fname = field_name(f.name.as_deref().unwrap_or("unknown"));
      let td = f
        .r#type
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedDefinition("message field missing type".into()))?;
      let rust_type = type_mapper::rust_type(td)?;
      let tag = f
        .index
        .ok_or_else(|| GeneratorError::MalformedDefinition("message field missing index".into()))?;
      let wrap = field_wrap(td, own_fqn);
      Ok(FieldMeta {
        fname,
        rust_type,
        tag,
        wrap,
      })
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Derive + struct definition
  output.push_str("#[derive(Debug, Clone, Default)]\n");
  output.push_str(&format!("pub struct {} {{\n", name));
  for (i, f) in fields.iter().enumerate() {
    let meta = &field_metas[i];
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "  pub {}: Option<Box<{}>>,\n",
          meta.fname, meta.rust_type
        ));
      }
      _ => {
        output.push_str(&format!(
          "  pub {}: Option<{}>,\n",
          meta.fname, meta.rust_type
        ));
      }
    }
  }
  output.push_str("}\n\n");

  // impl decode + encode
  output.push_str(&format!("impl {} {{\n", name));

  // decode
  output.push_str("  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {\n");
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let end = reader.position() + length;\n");
  output.push_str("    let mut msg = Self::default();\n\n");
  output.push_str("    while reader.position() < end {\n");
  output.push_str("      let tag = reader.read_tag()?;\n");
  output.push_str("      if tag == 0 { break; }\n");
  output.push_str("      match tag {\n");

  for (i, f) in fields.iter().enumerate() {
    let meta = &field_metas[i];
    let td = f.r#type.as_ref().unwrap();
    let read_expr = type_mapper::read_expression(td, "reader")?;

    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "        {} => msg.{} = Some(Box::new({}?)),\n",
          meta.tag, meta.fname, read_expr
        ));
      }
      _ => {
        output.push_str(&format!(
          "        {} => msg.{} = Some({}?),\n",
          meta.tag, meta.fname, read_expr
        ));
      }
    }
  }

  output.push_str("        _ => { reader.skip(end - reader.position())?; }\n");
  output.push_str("      }\n");
  output.push_str("    }\n");
  output.push_str("    Ok(msg)\n");
  output.push_str("  }\n\n");

  // encode
  output.push_str("  pub fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str("    let pos = writer.reserve_message_length();\n");

  for (i, f) in fields.iter().enumerate() {
    let meta = &field_metas[i];
    let td = f.r#type.as_ref().unwrap();
    let kind = td.kind.unwrap_or(TypeKind::Unknown);

    match meta.wrap {
      FieldWrap::Boxed => {
        // Box<T> — deref through box
        output.push_str(&format!(
          "    if let Some(ref v) = self.{} {{\n",
          meta.fname
        ));
        output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
        let write_stmt = type_mapper::write_expression(td, "v", "writer")?;
        output.push_str(&format!("      {};\n", write_stmt));
        output.push_str("    }\n");
      }
      _ => {
        // Determine the right pattern binding
        let is_scalar_copy = kind.is_scalar() && scalar_needs_deref(kind);

        if is_scalar_copy {
          // Scalar copy types: use `Some(v)` with value, write `v` directly
          output.push_str(&format!("    if let Some(v) = self.{} {{\n", meta.fname));
          output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
          let write_stmt = type_mapper::write_expression(td, "v", "writer")?;
          output.push_str(&format!("      {};\n", write_stmt));
          output.push_str("    }\n");
        } else {
          // Reference types (String, Vec, HashMap, defined types): use `Some(ref v)`
          output.push_str(&format!(
            "    if let Some(ref v) = self.{} {{\n",
            meta.fname
          ));
          output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
          let write_stmt = type_mapper::write_expression(td, "v", "writer")?;
          output.push_str(&format!("      {};\n", write_stmt));
          output.push_str("    }\n");
        }
      }
    }
  }

  output.push_str("    writer.write_end_marker();\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  Ok(())
}
