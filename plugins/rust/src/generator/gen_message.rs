use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeDescriptor, TypeKind};

use super::naming::{field_name, type_name};
use super::type_mapper;
use super::{emit_deprecated, emit_doc_comment, LifetimeAnalysis};

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
fn field_wrap(field_type: &TypeDescriptor<'_>, own_fqn: &str) -> FieldWrap {
  let kind = field_type.kind.unwrap_or(TypeKind::Unknown);

  // Direct self-reference: field's DEFINED fqn == own fqn
  if kind == TypeKind::Defined && field_type.defined_fqn.as_deref() == Some(own_fqn) {
    return FieldWrap::Boxed;
  }

  // Array of self: element is DEFINED with fqn == own fqn — Vec provides indirection
  if kind == TypeKind::Array {
    if let Some(elem) = field_type.array_element.as_ref() {
      if elem.kind == Some(TypeKind::Defined) && elem.defined_fqn.as_deref() == Some(own_fqn) {
        return FieldWrap::VecIndirect;
      }
    }
  }

  FieldWrap::Plain
}

/// Generate Rust code for a message definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let own_fqn = def.fqn.as_deref().unwrap_or("");

  let message_def = def
    .message_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("message missing message_def".into()))?;

  let fields = message_def.fields.as_deref().unwrap_or(&[]);
  let has_lifetime = analysis.lifetime_fqns.contains(own_fqn);

  let lt = if has_lifetime { "<'buf>" } else { "" };

  // Pre-compute field metadata
  struct FieldMeta<'a> {
    fname: String,
    cow_type: String,
    tag: u32,
    wrap: FieldWrap,
    td: &'a TypeDescriptor<'a>,
    kind: TypeKind,
    needs_owned: bool,
  }

  let field_metas: Vec<FieldMeta<'_>> = fields
    .iter()
    .map(|f| {
      let fname = field_name(f.name.as_deref().unwrap_or("unknown"));
      let td = f
        .r#type
        .as_ref()
        .ok_or_else(|| GeneratorError::MalformedDefinition("message field missing type".into()))?;
      let cow_type = type_mapper::rust_type(td, analysis)?;
      let tag = f
        .index
        .ok_or_else(|| GeneratorError::MalformedDefinition("message field missing index".into()))?;
      let wrap = field_wrap(td, own_fqn);
      Ok(FieldMeta {
        fname,
        cow_type,
        tag,
        wrap,
        td,
        kind: td.kind.unwrap_or(TypeKind::Unknown),
        needs_owned: analysis.type_needs_lifetime(td),
      })
    })
    .collect::<Result<Vec<_>, GeneratorError>>()?;

  // ── Doc comment + deprecated ──────────────────────────────────
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // ── Derive + struct definition ────────────────────────────────
  output.push_str("#[derive(Debug, Clone, Default)]\n");
  output.push_str(&format!("pub struct {}{} {{\n", name, lt));
  for (f, meta) in fields.iter().zip(&field_metas) {
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "  pub {}: Option<Box<{}>>,\n",
          meta.fname, meta.cow_type
        ));
      }
      _ => {
        output.push_str(&format!(
          "  pub {}: Option<{}>,\n",
          meta.fname, meta.cow_type
        ));
      }
    }
  }
  output.push_str("}\n\n");

  // ── Type alias ────────────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!("pub type {}Owned = {}<'static>;\n\n", name, name));
  }

  // ── into_owned() (only when has_lifetime) ─────────────────────
  if has_lifetime {
    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
    output.push_str(&format!("  pub fn into_owned(self) -> {}Owned {{\n", name));
    output.push_str(&format!("    {} {{\n", name));
    for meta in &field_metas {
      match meta.wrap {
        FieldWrap::Boxed => {
          if meta.needs_owned {
            output.push_str(&format!(
              "      {}: self.{}.map(|v| Box::new(v.into_owned())),\n",
              meta.fname, meta.fname
            ));
          } else {
            output.push_str(&format!("      {}: self.{},\n", meta.fname, meta.fname));
          }
        }
        _ => {
          if meta.needs_owned {
            let inner_expr = type_mapper::into_owned_expression(meta.td, "v", analysis)?;
            output.push_str(&format!(
              "      {}: self.{}.map(|v| {}),\n",
              meta.fname, meta.fname, inner_expr
            ));
          } else {
            output.push_str(&format!("      {}: self.{},\n", meta.fname, meta.fname));
          }
        }
      }
    }
    output.push_str("    }\n");
    output.push_str("  }\n");
    output.push_str("}\n\n");
  }

  // ── impl BebopEncode ──────────────────────────────────────────
  output.push_str(&format!("impl{} BebopEncode for {}{} {{\n", lt, name, lt));

  // encode()
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str("    let pos = writer.reserve_message_length();\n");

  for meta in &field_metas {
    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "    if let Some(ref v) = self.{} {{\n",
          meta.fname
        ));
        output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
        let write_stmt = type_mapper::write_expression(meta.td, "v", "writer", analysis)?;
        output.push_str(&format!("      {};\n", write_stmt));
        output.push_str("    }\n");
      }
      _ => {
        let is_scalar_copy =
          meta.kind.is_scalar() && meta.kind != TypeKind::String && meta.kind != TypeKind::Uuid;

        if is_scalar_copy {
          output.push_str(&format!("    if let Some(v) = self.{} {{\n", meta.fname));
          output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
          let write_stmt = type_mapper::write_expression(meta.td, "v", "writer", analysis)?;
          output.push_str(&format!("      {};\n", write_stmt));
          output.push_str("    }\n");
        } else {
          output.push_str(&format!(
            "    if let Some(ref v) = self.{} {{\n",
            meta.fname
          ));
          output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
          let write_stmt = type_mapper::write_expression(meta.td, "v", "writer", analysis)?;
          output.push_str(&format!("      {};\n", write_stmt));
          output.push_str("    }\n");
        }
      }
    }
  }

  output.push_str("    writer.write_end_marker();\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  output.push_str("    let mut size = wire::WIRE_MESSAGE_BASE_SIZE;\n");
  for meta in &field_metas {
    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "    if let Some(ref v) = self.{} {{\n",
          meta.fname
        ));
        let size_expr = type_mapper::encoded_size_expression(meta.td, "v", analysis)?;
        output.push_str(&format!(
          "      size += wire::tagged_size({});\n",
          size_expr
        ));
        output.push_str("    }\n");
      }
      _ => {
        let is_scalar_copy =
          meta.kind.is_scalar() && meta.kind != TypeKind::String && meta.kind != TypeKind::Uuid;

        if is_scalar_copy {
          output.push_str(&format!("    if let Some(v) = self.{} {{\n", meta.fname));
          let size_expr = type_mapper::encoded_size_expression(meta.td, "v", analysis)?;
          output.push_str(&format!(
            "      size += wire::tagged_size({});\n",
            size_expr
          ));
          output.push_str("    }\n");
        } else {
          output.push_str(&format!(
            "    if let Some(ref v) = self.{} {{\n",
            meta.fname
          ));
          let size_expr = type_mapper::encoded_size_expression(meta.td, "v", analysis)?;
          output.push_str(&format!(
            "      size += wire::tagged_size({});\n",
            size_expr
          ));
          output.push_str("    }\n");
        }
      }
    }
  }
  output.push_str("    size\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {}{} {{\n",
    name, lt
  ));
  output.push_str("  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {\n");
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let end = reader.position() + length;\n");
  output.push_str("    let mut msg = Self::default();\n\n");
  output.push_str("    while reader.position() < end {\n");
  output.push_str("      let tag = reader.read_tag()?;\n");
  output.push_str("      if tag == 0 { break; }\n");
  output.push_str("      match tag {\n");

  for meta in &field_metas {
    match meta.wrap {
      FieldWrap::Boxed => {
        let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
        output.push_str(&format!(
          "        {} => msg.{} = Some(Box::new({}?)),\n",
          meta.tag, meta.fname, read_expr
        ));
      }
      _ => {
        if let Some(read_expr) = type_mapper::borrowed_cow_read_expression(meta.td, "reader") {
          output.push_str(&format!(
            "        {} => msg.{} = Some({}),\n",
            meta.tag, meta.fname, read_expr
          ));
        } else {
          let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
          output.push_str(&format!(
            "        {} => msg.{} = Some({}?),\n",
            meta.tag, meta.fname, read_expr
          ));
        }
      }
    }
  }

  output.push_str("        _ => { reader.skip(end - reader.position())?; }\n");
  output.push_str("      }\n");
  output.push_str("    }\n");
  output.push_str("    Ok(msg)\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  Ok(())
}
