use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

/// Generate Rust code for a union definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate union {}\n", name));
  output.push_str("//\n");
  output.push_str("// Unions need:\n");
  output.push_str("//   - Rust enum with a variant per branch\n");
  output.push_str("//   - Unknown(u8, Vec<u8>) variant for forward compatibility\n");
  output.push_str("//   - impl decode: read length + discriminator byte, match to branch\n");
  output.push_str("//   - impl encode: write length + discriminator + branch payload\n");
  output.push_str("//   - impl encoded_size: 4 (length) + 1 (discriminator) + branch size\n");
  output.push_str("//   - Handle inline branches (type defined inside the union)\n");
  output.push_str("//   - Handle type-reference branches (references existing type)\n");
  output.push_str("//   - Handle nested definitions for inline branches\n");
  output.push('\n');

  if let Some(ref union_def) = def.union_def {
    if let Some(ref branches) = union_def.branches {
      eprintln!(
        "[bebopc-gen-rust]   union {} has {} branches",
        name,
        branches.len(),
      );
      for b in branches {
        let disc = b.discriminator.unwrap_or(0);
        let bname = b.name.as_deref().or(b.inline_fqn.as_deref()).unwrap_or("?");
        eprintln!("[bebopc-gen-rust]     {}({})", bname, disc);
      }
    }
  }

  Ok(())
}
