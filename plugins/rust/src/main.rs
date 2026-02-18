mod error;
mod generated;
mod generator;
mod wire;

use std::io::{self, Read, Write};

use error::GeneratorError;
use generated::{CodeGeneratorRequest, CodeGeneratorResponse, GeneratedFile};
use generator::RustGenerator;
use wire::{BebopReader, BebopWriter};

fn read_all_stdin() -> io::Result<Vec<u8>> {
  let mut buf = Vec::new();
  io::stdin().read_to_end(&mut buf)?;
  Ok(buf)
}

fn run() -> Result<CodeGeneratorResponse, GeneratorError> {
  let input = read_all_stdin()?;
  if input.is_empty() {
    return Err(GeneratorError::EmptyInput);
  }

  eprintln!("[bebopc-gen-rust] read {} bytes from stdin", input.len());

  let mut reader = BebopReader::new(&input);
  let request = CodeGeneratorRequest::decode(&mut reader)?;

  // Log compiler version
  if let Some(ref v) = request.compiler_version {
    eprintln!("[bebopc-gen-rust] compiler version: {}", v);
  }

  // Log parameters
  if let Some(ref param) = request.parameter {
    eprintln!("[bebopc-gen-rust] parameter: {}", param);
  }

  // Log host options
  if let Some(ref opts) = request.host_options {
    for (k, v) in opts {
      eprintln!("[bebopc-gen-rust] option: {} = {}", k, v);
    }
  }

  // Log files to generate
  let files_to_generate = request.files_to_generate.as_deref().unwrap_or(&[]);
  eprintln!(
    "[bebopc-gen-rust] files_to_generate: {:?}",
    files_to_generate
  );

  // Log schemas
  let schemas = request.schemas.as_deref().unwrap_or(&[]);
  eprintln!("[bebopc-gen-rust] {} schema(s) received", schemas.len());

  for schema in schemas {
    let path = schema.path.as_deref().unwrap_or("<no path>");
    let def_count = schema.definitions.as_ref().map_or(0, |d| d.len());
    let import_count = schema.imports.as_ref().map_or(0, |i| i.len());
    eprintln!(
      "[bebopc-gen-rust]   schema: {} ({} definitions, {} imports)",
      path, def_count, import_count,
    );
  }

  // Build the set of files we should generate
  let file_set: std::collections::HashSet<&str> =
    files_to_generate.iter().map(|s| s.as_str()).collect();

  let generator = RustGenerator::new(request.compiler_version);

  let mut generated_files = Vec::new();

  for schema in schemas {
    let path = match schema.path.as_deref() {
      Some(p) if file_set.contains(p) => p,
      _ => continue,
    };

    eprintln!("[bebopc-gen-rust] generating for: {}", path);

    // Derive output filename: foo/bar.bop -> bar.bb.rs
    let file_stem = std::path::Path::new(path)
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("output");
    let output_name = format!("{}.bb.rs", file_stem);

    let code = generator.generate(schema)?;

    generated_files.push(GeneratedFile {
      name: Some(output_name),
      content: Some(code),
      ..Default::default()
    });
  }

  eprintln!(
    "[bebopc-gen-rust] generated {} file(s)",
    generated_files.len()
  );

  Ok(CodeGeneratorResponse {
    error: None,
    files: if generated_files.is_empty() {
      None
    } else {
      Some(generated_files)
    },
    diagnostics: None,
  })
}

fn main() {
  let response = match run() {
    Ok(resp) => resp,
    Err(e) => {
      eprintln!("[bebopc-gen-rust] error: {}", e);
      CodeGeneratorResponse {
        error: Some(format!("bebopc-gen-rust: {}", e)),
        files: None,
        diagnostics: None,
      }
    }
  };

  let mut writer = BebopWriter::new();
  response.encode(&mut writer);

  if let Err(e) = io::stdout().write_all(&writer.into_bytes()) {
    eprintln!("[bebopc-gen-rust] failed to write response: {}", e);
    std::process::exit(1);
  }
}
