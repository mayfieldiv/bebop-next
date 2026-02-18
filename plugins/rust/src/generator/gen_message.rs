use crate::descriptor::DefinitionDescriptor;
use crate::error::GeneratorError;

/// Generate Rust code for a message definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate message {}\n", name));
  output.push_str("//\n");
  output.push_str("// Messages need:\n");
  output.push_str("//   - pub struct with all fields as Option<T> (tagged encoding)\n");
  output.push_str("//   - impl Default\n");
  output.push_str("//   - impl decode: read message length, then tag loop with match\n");
  output.push_str("//   - impl encode: write tag + value for each Some field, end marker\n");
  output.push_str("//   - impl encoded_size: 5 (length + end marker) + sum of present fields\n");
  output.push_str("//   - Handle unknown tags by skipping to end\n");
  output.push_str("//   - Handle nested definitions\n");
  output.push_str("//   - Handle doc comments and @deprecated\n");
  output.push('\n');

  if let Some(ref message_def) = def.message_def {
    if let Some(ref fields) = message_def.fields {
      eprintln!(
        "[bebopc-gen-rust]   message {} has {} fields",
        name,
        fields.len(),
      );
      for f in fields {
        let fname = f.name.as_deref().unwrap_or("?");
        let findex = f.index.unwrap_or(0);
        let fkind = f
          .field_type
          .as_ref()
          .and_then(|t| t.kind)
          .map_or("?".to_string(), |k| format!("{:?}", k));
        eprintln!("[bebopc-gen-rust]     {}({}): {}", fname, findex, fkind);
      }
    }
  }

  Ok(())
}
