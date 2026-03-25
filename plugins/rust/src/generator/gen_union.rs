use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

use super::naming::{fqn_to_type_name, to_snake_case, type_name};
use super::{
  emit_deprecated, emit_doc_comment, has_decorator, visibility_keyword, GeneratorOptions,
  LifetimeAnalysis, FORWARD_COMPATIBLE,
};

/// Generate Rust code for a union definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
  options: &GeneratorOptions,
  analysis: &LifetimeAnalysis,
) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));
  let fqn = def.fqn.as_deref().unwrap_or("");

  let union_def = def
    .union_def
    .as_ref()
    .ok_or_else(|| GeneratorError::MalformedDefinition("union missing union_def".into()))?;

  let branches = union_def.branches.as_deref().unwrap_or(&[]);

  // Collect branch info
  struct BranchInfo {
    variant: String,
    disc: u8,
    inner_type: String,
    inner_fqn: Option<String>,
  }

  let branch_infos: Vec<BranchInfo> = branches
    .iter()
    .map(|b| {
      let disc = b.discriminator.unwrap_or(0);
      let (variant, inner_type, inner_fqn) = if let Some(ref fqn) = b.inline_fqn {
        let t = fqn_to_type_name(fqn);
        (t.clone(), t, Some(fqn.to_string()))
      } else if let Some(ref bname) = b.name {
        let fqn = b.type_ref_fqn.as_ref();
        let t = if let Some(fqn) = fqn {
          fqn_to_type_name(fqn)
        } else {
          type_name(bname)
        };
        (type_name(bname), t, fqn.map(|f| f.to_string()))
      } else {
        ("Unknown".to_string(), "Unknown".to_string(), None)
      };
      BranchInfo {
        variant,
        disc,
        inner_type,
        inner_fqn,
      }
    })
    .collect();

  let vis = visibility_keyword(def, options);
  let is_forward_compatible = has_decorator(def, FORWARD_COMPATIBLE);
  let has_lifetime = analysis.lifetime_fqns.contains(fqn);
  let lt = if has_lifetime { "<'buf>" } else { "" };

  // ── Doc comment + deprecated ──────────────────────────────────
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // ── Enum definition ───────────────────────────────────────────
  let mut derives = vec!["Debug", "Clone", "PartialEq"];
  if analysis.can_derive_eq(fqn) {
    derives.push("Eq");
  }
  if analysis.can_derive_hash(fqn) {
    derives.push("Hash");
  }
  options.serde.emit_derive(output);
  options
    .serde
    .emit_type_attr(output, "tag = \"type\", content = \"value\"");
  output.push_str(&format!("#[derive({})]\n", derives.join(", ")));
  output.push_str(&format!("{} enum {}{} {{\n", vis, name, lt));
  for b in &branch_infos {
    let inner_lt = if let Some(ref fqn) = b.inner_fqn {
      if analysis.lifetime_fqns.contains(fqn) {
        "<'buf>"
      } else {
        ""
      }
    } else {
      ""
    };
    output.push_str(&format!("  {}({}{}),\n", b.variant, b.inner_type, inner_lt));
  }
  if is_forward_compatible {
    options.serde.emit_field_attr(output, "skip");
    output.push_str("  Unknown(u8, borrow::Cow<'buf, [u8]>),\n");
  }
  output.push_str("}\n\n");

  // ── Type alias + into_owned() (only when lifetime is present) ──
  if has_lifetime {
    output.push_str(&format!(
      "{} type {}Owned = {}<'static>;\n\n",
      vis, name, name
    ));

    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
    output.push_str(&format!("  pub fn into_owned(self) -> {}Owned {{\n", name));
    output.push_str("    match self {\n");
    for b in &branch_infos {
      let has_lt = b
        .inner_fqn
        .as_ref()
        .is_some_and(|fqn| analysis.lifetime_fqns.contains(fqn));
      if has_lt {
        output.push_str(&format!(
          "      Self::{}(inner) => {}::{}(inner.into_owned()),\n",
          b.variant, name, b.variant
        ));
      } else {
        output.push_str(&format!(
          "      Self::{}(inner) => {}::{}(inner),\n",
          b.variant, name, b.variant
        ));
      }
    }
    if is_forward_compatible {
      output.push_str(&format!(
        "      Self::Unknown(disc, data) => {}::Unknown(disc, borrow::Cow::Owned(data.into_owned())),\n",
        name
      ));
    }
    output.push_str("    }\n");
    output.push_str("  }\n");
    output.push_str("}\n\n");
  }

  // ── impl bebop::BebopEncode ──────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!(
      "impl<'buf> bebop::BebopEncode for {}<'buf> {{\n",
      name
    ));
  } else {
    output.push_str(&format!("impl bebop::BebopEncode for {} {{\n", name));
  }

  // encode()
  output.push_str("  fn encode(&self, writer: &mut bebop::BebopWriter) {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_start:{})\n",
    name
  ));
  output.push_str("    let pos = writer.reserve_message_length();\n");
  output.push_str("    match self {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      Self::{}(inner) => {{ writer.write_byte({}); inner.encode(writer); }}\n",
      b.variant, b.disc
    ));
  }
  if is_forward_compatible {
    output.push_str(&format!(
      "      // @@bebop_insertion_point(encode_switch:{})\n",
      name
    ));
    output.push_str(
      "      Self::Unknown(disc, data) => { writer.write_byte(*disc); writer.write_raw(data); }\n",
    );
  }
  output.push_str("    }\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  output.push_str("    bebop::wire_size::WIRE_LEN_PREFIX_SIZE + match self {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      Self::{}(inner) => bebop::wire_size::tagged_size(inner.encoded_size()),\n",
      b.variant
    ));
  }
  if is_forward_compatible {
    output.push_str("      Self::Unknown(_, data) => bebop::wire_size::tagged_size(data.len()),\n");
  }
  output.push_str("    }\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl bebop::BebopDecode ──────────────────────────────────────────
  if has_lifetime {
    output.push_str(&format!(
      "impl<'buf> bebop::BebopDecode<'buf> for {}<'buf> {{\n",
      name
    ));
  } else {
    output.push_str(&format!(
      "impl<'buf> bebop::BebopDecode<'buf> for {} {{\n",
      name
    ));
  }
  output.push_str("  #[inline]\n");
  output.push_str(
    "  fn decode(reader: &mut bebop::BebopReader<'buf>) -> result::Result<Self, bebop::DecodeError> {\n",
  );
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let start = reader.position();\n");
  output.push_str("    let discriminator = reader.read_byte()?;\n");
  output.push_str("    let value = match discriminator {\n");
  for b in &branch_infos {
    // Use snake_case variant name as field_name for error context.
    let branch_field = to_snake_case(&b.variant);
    output.push_str(&format!(
      "      {} => result::Result::Ok(Self::{}({}::decode(reader).for_field(\"{}\", \"{}\")?)),\n",
      b.disc, b.variant, b.inner_type, name, branch_field
    ));
  }
  if is_forward_compatible {
    output.push_str(&format!(
      "      // @@bebop_insertion_point(decode_switch:{})\n",
      name
    ));
    output.push_str("      _ => {\n");
    output.push_str("        let remaining = length - (reader.position() - start);\n");
    output.push_str("        let data = reader.read_raw_bytes(remaining)?;\n");
    output.push_str(
      "        result::Result::Ok(Self::Unknown(discriminator, borrow::Cow::Borrowed(data)))\n",
    );
    output.push_str("      }\n");
  } else {
    output.push_str(&format!(
      "      _ => result::Result::Err(bebop::DecodeError::InvalidUnion {{ type_name: \"{}\", discriminator }}),\n",
      name
    ));
  }
  output.push_str("    };\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  output.push_str("    value\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  if has_lifetime {
    output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
  } else {
    output.push_str(&format!("impl {} {{\n", name));
  }
  output.push_str(&format!(
    "  // @@bebop_insertion_point(union_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  Ok(())
}
