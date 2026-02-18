/// Compiler version.
/// Follows semantic versioning. The `suffix` field distinguishes pre-release
/// versions (`alpha.1`, `beta.2`, `rc.1`). Empty suffix is a stable release.
#[derive(Debug, Clone)]
pub struct Version {
  pub major: i32,
  pub minor: i32,
  pub patch: i32,
/// Pre-release suffix (`alpha.1`, `rc.2`). Empty for stable releases.
  pub suffix: String,
}

impl Version {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let major = reader.read_i32()?;
    let minor = reader.read_i32()?;
    let patch = reader.read_i32()?;
    let suffix = reader.read_string()?;
    Ok(Version {
      major,
      minor,
      patch,
      suffix,
    })
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_i32(self.major);
    writer.write_i32(self.minor);
    writer.write_i32(self.patch);
    writer.write_string(&self.suffix);
  }
}

/// Input to a code generator plugin.
/// The compiler writes an encoded CodeGeneratorRequest to the plugin's stdin.
/// Generate code only for files in `files_to_generate`, using the full
/// `schemas` array for type resolution.
#[derive(Debug, Clone, Default)]
pub struct CodeGeneratorRequest {
/// The .bop files to generate code for (explicitly listed on command line).
/// Each file's descriptor is included in `schemas`.
  pub files_to_generate: Option<Vec<String>>,
/// Generator-specific parameter from `--${NAME}_opt=PARAM` or embedded
/// in the output path. Format is plugin-defined (commonly key=value pairs).
  pub parameter: Option<String>,
/// Version of the compiler invoking the plugin. Use to detect
/// incompatibilities or enable version-specific behavior.
  pub compiler_version: Option<Version>,
/// SchemaDescriptors for all files in `files_to_generate` plus their
/// imports. Schemas appear in topological order: dependencies before
/// dependents. Type FQNs are fully resolved.
/// Iterate schemas, check if `schema.path` is in `files_to_generate`,
/// and generate code only for those files. The rest are for type resolution.
  pub schemas: Option<Vec<SchemaDescriptor>>,
/// Host compiler options passed to bebopc. Use to adjust output based
/// on global settings.
  pub host_options: Option<std::collections::HashMap<String, String>>,
}

impl CodeGeneratorRequest {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.files_to_generate = Some(reader.read_array(|_r| _r.read_string())?),
        2 => msg.parameter = Some(reader.read_string()?),
        3 => msg.compiler_version = Some(Version::decode(reader)?),
        4 => msg.schemas = Some(reader.read_array(SchemaDescriptor::decode)?),
        5 => msg.host_options = Some(reader.read_map(|_r| Ok((_r.read_string()?, _r.read_string()?)))?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.files_to_generate {
      writer.write_tag(1);
      writer.write_array(&v, |_w, _el| _w.write_string(_el));
    }
    if let Some(ref v) = self.parameter {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.compiler_version {
      writer.write_tag(3);
      v.encode(writer);
    }
    if let Some(ref v) = self.schemas {
      writer.write_tag(4);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.host_options {
      writer.write_tag(5);
      writer.write_map(&v, |_w, _k, _v| { _w.write_string(&_k); _w.write_string(&_v); });
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Severity level for plugin diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSeverity(pub u8);

#[allow(non_upper_case_globals)]
impl DiagnosticSeverity {
  pub const ERROR: Self = Self(0);
  pub const WARNING: Self = Self(1);
  pub const INFO: Self = Self(2);
  pub const HINT: Self = Self(3);
}

impl DiagnosticSeverity {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self(reader.read_byte()?))
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    writer.write_byte(self.0);
  }

  pub fn encoded_size(&self) -> usize { 1 }
}

/// Diagnostic message from a plugin.
/// Report errors, warnings, and info about schemas being processed.
/// Displayed to the user alongside the plugin's name.
#[derive(Debug, Clone, Default)]
pub struct Diagnostic {
  pub severity: Option<DiagnosticSeverity>,
/// Human-readable diagnostic text.
  pub text: Option<String>,
/// Optional hint for fixing the issue.
  pub hint: Option<String>,
/// Source file path this diagnostic relates to.
  pub file: Option<String>,
/// Source location as `[start_line, start_col, end_line, end_col]`.
/// 1-based. Absent if not applicable.
  pub span: Option<[i32; 4]>,
}

impl Diagnostic {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.severity = Some(DiagnosticSeverity::decode(reader)?),
        2 => msg.text = Some(reader.read_string()?),
        3 => msg.hint = Some(reader.read_string()?),
        4 => msg.file = Some(reader.read_string()?),
        5 => msg.span = Some(reader.read_fixed_i32_array::<4>()?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.severity {
      writer.write_tag(1);
      v.encode(writer);
    }
    if let Some(ref v) = self.text {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.hint {
      writer.write_tag(3);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.file {
      writer.write_tag(4);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.span {
      writer.write_tag(5);
      writer.write_fixed_i32_array::<4>(&v);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// A generated file.
/// Either a complete new file or an insertion into an existing file at a
/// marked insertion point.
#[derive(Debug, Clone, Default)]
pub struct GeneratedFile {
/// Output path relative to output directory. No `..` components or
/// leading `/`. Use `/` as separator on all platforms.
/// When `insertion_point` is set, names the file to insert into (must
/// be generated by a prior plugin in the same invocation).
/// When omitted, content appends to the previous file. Allows generators
/// to stream large files in chunks.
  pub name: Option<String>,
/// Insertion point name for extending another plugin's output.
/// Target file must contain:
/// ```
/// // @@bebopc_insertion_point(NAME)
/// ```
/// Content inserts above this marker. Multiple insertions to the same
/// point appear in plugin execution order.
/// When set, `name` must also be set to identify the target file.
  pub insertion_point: Option<String>,
/// File contents (complete file or insertion fragment).
/// For insertions, typically includes a trailing newline.
  pub content: Option<String>,
/// Source mapping connecting generated code to source schemas.
/// Optional; enables IDE features like go-to-definition.
  pub generated_code_info: Option<SourceCodeInfo>,
}

impl GeneratedFile {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.name = Some(reader.read_string()?),
        2 => msg.insertion_point = Some(reader.read_string()?),
        3 => msg.content = Some(reader.read_string()?),
        4 => msg.generated_code_info = Some(SourceCodeInfo::decode(reader)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.name {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.insertion_point {
      writer.write_tag(2);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.content {
      writer.write_tag(3);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.generated_code_info {
      writer.write_tag(4);
      v.encode(writer);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Output from a code generator plugin.
/// Write an encoded CodeGeneratorResponse to stdout. The compiler reads it
/// and writes generated files to the output directory.
/// Error handling:
/// - Fatal plugin errors: write to stderr, exit non-zero
/// - Schema problems: set `error` field, exit zero
/// - Success: populate `files`, exit zero
/// The distinction matters: stderr + non-zero indicates a plugin bug or
/// environment problem. The `error` field indicates problems in .bop files
/// that prevent correct code generation.
#[derive(Debug, Clone, Default)]
pub struct CodeGeneratorResponse {
/// Error message. If non-empty, code generation failed.
/// Set for schema problems that prevent generating correct code. Exit
/// with status zero.
/// For plugin bugs or environment problems (can't read input, out of
/// memory), write to stderr and exit non-zero instead.
  pub error: Option<String>,
/// Generated files to write to the output directory.
/// Written in array order. Later files with `insertion_point` can
/// extend earlier files in the same response.
  pub files: Option<Vec<GeneratedFile>>,
/// Diagnostics to report. Displayed even on success (for warnings/info).
  pub diagnostics: Option<Vec<Diagnostic>>,
}

impl CodeGeneratorResponse {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut msg = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 { break; }
      match tag {
        1 => msg.error = Some(reader.read_string()?),
        2 => msg.files = Some(reader.read_array(GeneratedFile::decode)?),
        3 => msg.diagnostics = Some(reader.read_array(Diagnostic::decode)?),
        _ => { reader.skip(end - reader.position())?; }
      }
    }
    Ok(msg)
  }

  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref v) = self.error {
      writer.write_tag(1);
      writer.write_string(&v);
    }
    if let Some(ref v) = self.files {
      writer.write_tag(2);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    if let Some(ref v) = self.diagnostics {
      writer.write_tag(3);
      writer.write_array(&v, |_w, _el| _el.encode(_w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

