use crate::descriptor::DefinitionDescriptor;
use crate::error::GeneratorError;

/// Generate Rust code for an enum definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate enum {}\n", name));
  output.push_str("//\n");
  output.push_str("// Enums need:\n");
  output
    .push_str("//   - Newtype struct with RawValue (matching the base_type: u8, u16, u32, etc.)\n");
  output.push_str("//   - Associated constants for each member\n");
  output.push_str("//   - impl decode: read the base type integer\n");
  output.push_str("//   - impl encode: write the base type integer\n");
  output.push_str("//   - impl encoded_size: constant based on base type\n");
  output.push_str("//   - For @flags enums: bitwise OR support (BitOr, BitAnd, etc.)\n");
  output.push_str("//   - Handle @deprecated decorator on members\n");
  output.push_str("//   - Handle doc comments from documentation field\n");
  output.push('\n');

  // Log details about what we received
  if let Some(ref enum_def) = def.enum_def {
    if let Some(ref members) = enum_def.members {
      eprintln!(
        "[bebopc-gen-rust]   enum {} has {} members, is_flags={:?}",
        name,
        members.len(),
        enum_def.is_flags,
      );
      for m in members {
        let mname = m.name.as_deref().unwrap_or("?");
        let mvalue = m.value.unwrap_or(0);
        eprintln!("[bebopc-gen-rust]     {} = {}", mname, mvalue);
      }
    }
  }

  Ok(())
}
