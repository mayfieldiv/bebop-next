use crate::error::GeneratorError;
use crate::generated::DefinitionDescriptor;

/// Generate Rust code for a const definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate const {}\n", name));
  output.push_str("//\n");
  output.push_str("// Constants need:\n");
  output.push_str("//   - pub const NAME: Type = literal_value;\n");
  output.push_str("//   - Map LiteralValue to Rust literal syntax\n");
  output.push_str("//   - Handle string, int, float, bool, uuid literal kinds\n");
  output.push_str("//   - Handle env var substitution (raw_value vs string_value)\n");
  output.push('\n');

  if let Some(ref const_def) = def.const_def {
    let type_kind = const_def
      .r#type
      .as_ref()
      .and_then(|t| t.kind)
      .map_or("?".to_string(), |k| format!("{:?}", k));
    eprintln!("[bebopc-gen-rust]   const {}: {}", name, type_kind,);
  }

  Ok(())
}
