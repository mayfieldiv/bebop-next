use crate::descriptor::DefinitionDescriptor;
use crate::error::GeneratorError;

/// Generate Rust code for a struct definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate struct {}\n", name));
  output.push_str("//\n");
  output.push_str("// Structs need:\n");
  output.push_str("//   - pub struct with all fields (non-optional, positional encoding)\n");
  output.push_str("//   - impl decode: read fields in declaration order, no tags\n");
  output.push_str("//   - impl encode: write fields in declaration order\n");
  output.push_str("//   - impl encoded_size: sum of field sizes\n");
  output.push_str("//   - Derive Debug, Clone, PartialEq\n");
  output.push_str("//   - Handle fixed_size optimization (pre-allocate buffers)\n");
  output.push_str("//   - Handle is_mutable (affects field mutability)\n");
  output.push_str("//   - Handle nested definitions\n");
  output.push_str("//   - Handle doc comments and @deprecated\n");
  output.push('\n');

  if let Some(ref struct_def) = def.struct_def {
    if let Some(ref fields) = struct_def.fields {
      eprintln!(
        "[bebopc-gen-rust]   struct {} has {} fields, fixed_size={:?}",
        name,
        fields.len(),
        struct_def.fixed_size,
      );
      for f in fields {
        let fname = f.name.as_deref().unwrap_or("?");
        let fkind = f
          .field_type
          .as_ref()
          .and_then(|t| t.kind)
          .map_or("?".to_string(), |k| format!("{:?}", k));
        eprintln!("[bebopc-gen-rust]     {}: {}", fname, fkind);
      }
    }
  }

  Ok(())
}
