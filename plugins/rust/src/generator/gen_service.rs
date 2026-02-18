use crate::descriptor::DefinitionDescriptor;
use crate::error::GeneratorError;

/// Generate Rust code for a service definition.
pub fn generate(def: &DefinitionDescriptor, output: &mut String) -> Result<(), GeneratorError> {
  let name = def.name.as_deref().unwrap_or("<unnamed>");

  output.push_str(&format!("// TODO: generate service {}\n", name));
  output.push_str("//\n");
  output.push_str("// Services need:\n");
  output.push_str("//   - Service definition enum with Method sub-enum\n");
  output.push_str("//   - Method enum: one variant per RPC method, with MurmurHash3 id\n");
  output.push_str("//   - Handler trait: async fn per method\n");
  output.push_str("//     - Unary: fn(Request, Context) -> Response\n");
  output.push_str("//     - ServerStream: fn(Request, Context) -> Stream<Response>\n");
  output.push_str("//     - ClientStream: fn(Stream<Request>, Context) -> Response\n");
  output.push_str("//     - DuplexStream: fn(Stream<Request>, Context) -> Stream<Response>\n");
  output.push_str("//   - Client struct: typed RPC call methods\n");
  output.push_str("//   - Batch accessor for batched unary/server-stream calls\n");
  output.push_str("//   - Router registration extension\n");
  output.push('\n');

  if let Some(ref service_def) = def.service_def {
    if let Some(ref methods) = service_def.methods {
      eprintln!(
        "[bebopc-gen-rust]   service {} has {} methods",
        name,
        methods.len(),
      );
      for m in methods {
        let mname = m.name.as_deref().unwrap_or("?");
        let mtype = m
          .method_type
          .map_or("?".to_string(), |t| format!("{:?}", t));
        let mid = m.id.map_or("?".to_string(), |id| format!("0x{:X}", id));
        eprintln!("[bebopc-gen-rust]     {} ({}) id={}", mname, mtype, mid);
      }
    }
  }

  Ok(())
}
