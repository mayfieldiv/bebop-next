use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

use super::naming::{fqn_to_type_name, type_name};
use super::{emit_deprecated, emit_doc_comment};

/// Generate Rust code for a union definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = type_name(def.name.as_deref().unwrap_or("<unnamed>"));

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
  }

  let branch_infos: Vec<BranchInfo> = branches
    .iter()
    .map(|b| {
      let disc = b.discriminator.unwrap_or(0);
      let (variant, inner_type) = if let Some(ref fqn) = b.inline_fqn {
        // Inline branch: type name derived from the inline FQN
        let t = fqn_to_type_name(fqn);
        (t.clone(), t)
      } else if let Some(ref name) = b.name {
        // Type-reference branch
        let t = if let Some(ref fqn) = b.type_ref_fqn {
          fqn_to_type_name(fqn)
        } else {
          type_name(name)
        };
        (type_name(name), t)
      } else {
        ("Unknown".to_string(), "Unknown".to_string())
      };
      BranchInfo {
        variant,
        disc,
        inner_type,
      }
    })
    .collect();

  // Doc comment + deprecated
  emit_doc_comment(output, &def.documentation);
  emit_deprecated(output, &def.decorators);

  // Enum definition
  output.push_str("#[derive(Debug, Clone)]\n");
  output.push_str(&format!("pub enum {} {{\n", name));
  for b in &branch_infos {
    output.push_str(&format!("  {}({}),\n", b.variant, b.inner_type));
  }
  // Unknown variant for forward compatibility
  output.push_str("  Unknown(u8, Vec<u8>),\n");
  output.push_str("}\n\n");

  // impl block with decode/encode
  output.push_str(&format!("impl {} {{\n", name));

  // decode
  output.push_str("  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {\n");
  output.push_str("    let length = reader.read_message_length()? as usize;\n");
  output.push_str("    let start = reader.position();\n");
  output.push_str("    let discriminator = reader.read_byte()?;\n");
  output.push_str("    match discriminator {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      {} => Ok(Self::{}({}::decode(reader)?)),\n",
      b.disc, b.variant, b.inner_type
    ));
  }
  output.push_str("      _ => {\n");
  output.push_str("        let remaining = length - (reader.position() - start);\n");
  output.push_str("        let data = reader.read_bytes(remaining)?;\n");
  output.push_str("        Ok(Self::Unknown(discriminator, data))\n");
  output.push_str("      }\n");
  output.push_str("    }\n");
  output.push_str("  }\n\n");

  // encode
  output.push_str("  pub fn encode(&self, writer: &mut BebopWriter) {\n");
  output.push_str("    let pos = writer.reserve_message_length();\n");
  output.push_str("    match self {\n");
  for b in &branch_infos {
    output.push_str(&format!(
      "      Self::{}(inner) => {{ writer.write_byte({}); inner.encode(writer); }}\n",
      b.variant, b.disc
    ));
  }
  output.push_str("      Self::Unknown(disc, data) => { writer.write_byte(*disc); writer.write_raw(data); }\n");
  output.push_str("    }\n");
  output.push_str("    writer.fill_message_length(pos);\n");
  output.push_str("  }\n");

  output.push_str("}\n\n");

  Ok(())
}
