use crate::error::GeneratorError;
use crate::generated::{DefinitionDescriptor, TypeDescriptor, TypeKind};

use super::naming::{field_name, serde_field_rename, type_name};
use super::type_mapper;
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis, FORWARD_COMPATIBLE,
};

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
  options: &GeneratorOptions,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let own_fqn = def.fqn.as_deref().unwrap_or("");

  let message_def = def
    .message_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("message missing message_def".into()))?;

  let fields = message_def.fields.as_deref().unwrap_or(&[]);
  let vis = visibility_keyword(def, options);
  let is_forward_compatible = has_decorator(def, FORWARD_COMPATIBLE);
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

  impl FieldMeta<'_> {
    /// Whether the `if let` binding needs `ref` (non-Copy types and boxed fields).
    fn needs_ref(&self) -> bool {
      matches!(self.wrap, FieldWrap::Boxed)
        || !self.kind.is_scalar()
        || self.kind == TypeKind::String
    }
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
  let mut derives = vec!["Debug", "Clone", "Default", "PartialEq"];
  if analysis.can_derive_eq(own_fqn) {
    derives.push("Eq");
  }
  if analysis.can_derive_hash(own_fqn) {
    derives.push("Hash");
  }
  options.serde.emit_derive(output);
  output.push_str(&format!("#[derive({})]\n", derives.join(", ")));
  output.push_str(&format!("{} struct {}{} {{\n", vis, name, lt));
  for (f, meta) in fields.iter().zip(&field_metas) {
    emit_doc_comment(output, &f.documentation);
    emit_deprecated(output, &f.decorators);
    if let Some(rename) = serde_field_rename(f.name.as_deref().unwrap_or("")) {
      options
        .serde
        .emit_field_attr(output, &format!("rename = \"{}\"", rename));
    }
    match meta.wrap {
      FieldWrap::Boxed => {
        output.push_str(&format!(
          "  {} {}: option::Option<boxed::Box<{}>>,\n",
          vis, meta.fname, meta.cow_type
        ));
      }
      _ => {
        output.push_str(&format!(
          "  {} {}: option::Option<{}>,\n",
          vis, meta.fname, meta.cow_type
        ));
      }
    }
  }
  output.push_str("}\n\n");

  // ── Type alias ────────────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!(
      "{} type {}Owned = {}<'static>;\n\n",
      vis, name, name
    ));
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
              "      {}: self.{}.map(|v| boxed::Box::new(v.into_owned())),\n",
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

  // ── impl bebop::BebopEncode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl{} bebop::BebopEncode for {}{} {{\n",
    lt, name, lt
  ));

  // encode()
  output.push_str("  fn encode(&self, writer: &mut bebop::BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    name
  ));
  output.push_str("    let pos = writer.reserve_message_length();\n");
  output.push_str(
    "    // NOTE: Deprecated fields are currently encoded and decoded like normal fields.\n",
  );
  output.push_str(
    "    // The GRAMMAR.md spec says deprecated message fields should be skipped during\n",
  );
  output.push_str(
    "    // encoding and decoding. The C plugin (plugins/c/src/generator.c:3446) skips\n",
  );
  output
    .push_str("    // them on encode/size but still decodes them. The Swift plugin encodes them\n");
  output.push_str(
    "    // normally. This behavior should be revisited once the spec intent is clarified.\n",
  );

  for meta in &field_metas {
    let ref_kw = if meta.needs_ref() { "ref " } else { "" };
    output.push_str(&format!(
      "    if let option::Option::Some({}v) = self.{} {{\n",
      ref_kw, meta.fname
    ));
    output.push_str(&format!("      writer.write_tag({});\n", meta.tag));
    let write_stmt = type_mapper::write_expression(meta.td, "v", "writer", analysis)?;
    output.push_str(&format!("      {};\n", write_stmt));
    output.push_str("    }\n");
  }

  output.push_str("    writer.write_end_marker();\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  output.push_str("    let mut size = bebop::wire_size::WIRE_MESSAGE_BASE_SIZE;\n");
  for meta in &field_metas {
    let ref_kw = if meta.needs_ref() { "ref " } else { "" };
    output.push_str(&format!(
      "    if let option::Option::Some({}v) = self.{} {{\n",
      ref_kw, meta.fname
    ));
    let size_expr = type_mapper::encoded_size_expression(meta.td, "v", analysis)?;
    output.push_str(&format!(
      "      size += bebop::wire_size::tagged_size({});\n",
      size_expr
    ));
    output.push_str("    }\n");
  }
  output.push_str("    size\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl bebop::BebopDecode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl<'buf> bebop::BebopDecode<'buf> for {}{} {{\n",
    name, lt
  ));
  output.push_str("  #[inline]\n");
  output.push_str(
    "  fn decode(reader: &mut bebop::BebopReader<'buf>) -> result::Result<Self, bebop::DecodeError> {\n",
  );
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let end = reader.position() + length;\n");
  output.push_str("    let mut msg = <Self as default::Default>::default();\n\n");
  output.push_str("    while reader.position() < end {\n");
  output.push_str("      let tag = reader.read_tag()?;\n");
  output.push_str("      if tag == 0 { break; }\n");
  output.push_str("      match tag {\n");

  for meta in &field_metas {
    match meta.wrap {
      FieldWrap::Boxed => {
        let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
        output.push_str(&format!(
          "        {} => msg.{} = option::Option::Some(boxed::Box::new({}?)),\n",
          meta.tag, meta.fname, read_expr
        ));
      }
      _ => {
        if let Some(read_expr) = type_mapper::borrowed_cow_read_expression(meta.td, "reader") {
          output.push_str(&format!(
            "        {} => msg.{} = option::Option::Some({}),\n",
            meta.tag, meta.fname, read_expr
          ));
        } else {
          let read_expr = type_mapper::read_expression(meta.td, "reader", analysis)?;
          output.push_str(&format!(
            "        {} => msg.{} = option::Option::Some({}?),\n",
            meta.tag, meta.fname, read_expr
          ));
        }
      }
    }
  }

  if is_forward_compatible {
    output.push_str("        _ => { reader.skip(end - reader.position())?; }\n");
  } else {
    output.push_str(&format!(
      "        tag => {{ return result::Result::Err(bebop::DecodeError::InvalidField {{ type_name: \"{}\", tag }}); }}\n",
      name
    ));
  }
  output.push_str("      }\n");
  output.push_str("    }\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  output.push_str("    result::Result::Ok(msg)\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  output.push_str(&format!("impl{} {}{} {{\n", lt, name, lt));
  output.push_str(&format!(
    "  // @@bebop_insertion_point(message_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  Ok(())
}
