use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

use super::naming::{fqn_to_type_name, type_name};
use super::{emit_deprecated, emit_doc_comment, LifetimeAnalysis};

/// Generate Rust code for a union definition.
pub fn generate(
  def: &DefinitionDescriptor,
  output: &mut String,
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

  // Unions always have lifetime (Unknown variant uses Cow<'buf, [u8]>)
  let lt = "<'buf>";

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
  output.push_str(&format!("#[derive({})]\n", derives.join(", ")));
  output.push_str(&format!("pub enum {}{} {{\n", name, lt));
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
  // Unknown variant for forward compatibility
  output.push_str("  Unknown(u8, Cow<'buf, [u8]>),\n");
  output.push_str("}\n\n");

  // ── Type alias ────────────────────────────────────────────────
  output.push_str(&format!("pub type {}Owned = {}<'static>;\n\n", name, name));

  // ── into_owned() ──────────────────────────────────────────────
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
  output.push_str(&format!(
    "      Self::Unknown(disc, data) => {}::Unknown(disc, Cow::Owned(data.into_owned())),\n",
    name
  ));
  output.push_str("    }\n");
  output.push_str("  }\n");
  output.push_str("}\n\n");

  // ── impl BebopEncode ──────────────────────────────────────────
  output.push_str(&format!("impl<'buf> BebopEncode for {}<'buf> {{\n", name));

  // encode()
  output.push_str("  fn encode(&self, writer: &mut BebopWriter) {\n");
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
  output.push_str(&format!(
    "      // @@bebop_insertion_point(encode_switch:{})\n",
    name
  ));
  output.push_str(
    "      Self::Unknown(disc, data) => { writer.write_byte(*disc); writer.write_raw(data); }\n",
  );
  output.push_str("    }\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(encode_end:{})\n",
    name
  ));
  output.push_str("  }\n\n");

  // encoded_size()
  output.push_str("  fn encoded_size(&self) -> usize {\n");
  output.push_str("    wire::WIRE_LEN_PREFIX_SIZE + match self {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      Self::{}(inner) => wire::tagged_size(inner.encoded_size()),\n",
      b.variant
    ));
  }
  output.push_str("      Self::Unknown(_, data) => wire::tagged_size(data.len()),\n");
  output.push_str("    }\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  // ── impl BebopDecode ──────────────────────────────────────────
  output.push_str(&format!(
    "impl<'buf> BebopDecode<'buf> for {}<'buf> {{\n",
    name
  ));
  output.push_str("  fn decode(reader: &mut BebopReader<'buf>) -> Result<Self, DecodeError> {\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_start:{})\n",
    name
  ));
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let start = reader.position();\n");
  output.push_str("    let discriminator = reader.read_byte()?;\n");
  output.push_str("    let value = match discriminator {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      {} => Ok(Self::{}({}::decode(reader)?)),\n",
      b.disc, b.variant, b.inner_type
    ));
  }
  output.push_str(&format!(
    "      // @@bebop_insertion_point(decode_switch:{})\n",
    name
  ));
  output.push_str("      _ => {\n");
  output.push_str("        let remaining = length - (reader.position() - start);\n");
  output.push_str("        let data = reader.read_raw_bytes(remaining)?;\n");
  output.push_str("        Ok(Self::Unknown(discriminator, Cow::Borrowed(data)))\n");
  output.push_str("      }\n");
  output.push_str("    };\n");
  output.push_str(&format!(
    "    // @@bebop_insertion_point(decode_end:{})\n",
    name
  ));
  output.push_str("    value\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  output.push_str(&format!("impl<'buf> {}<'buf> {{\n", name));
  output.push_str(&format!(
    "  // @@bebop_insertion_point(union_scope:{})\n",
    name
  ));
  output.push_str("}\n\n");

  Ok(())
}
