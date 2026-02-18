use crate::wire::BebopWriter;

/// Severity level for plugin diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSeverity(pub u8);

#[allow(dead_code)]
impl DiagnosticSeverity {
  pub const ERROR: Self = Self(0);
  pub const WARNING: Self = Self(1);
  pub const INFO: Self = Self(2);
  pub const HINT: Self = Self(3);
}

/// Diagnostic message from a plugin (message on wire).
#[derive(Debug, Clone, Default)]
pub struct Diagnostic {
  pub severity: Option<DiagnosticSeverity>, // tag 1
  pub text: Option<String>,                 // tag 2
  pub hint: Option<String>,                 // tag 3
  pub file: Option<String>,                 // tag 4
  pub span: Option<[i32; 4]>,               // tag 5
}

impl Default for DiagnosticSeverity {
  fn default() -> Self {
    Self(0)
  }
}

impl Diagnostic {
  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref sev) = self.severity {
      writer.write_tag(1);
      writer.write_byte(sev.0);
    }
    if let Some(ref text) = self.text {
      writer.write_tag(2);
      writer.write_string(text);
    }
    if let Some(ref hint) = self.hint {
      writer.write_tag(3);
      writer.write_string(hint);
    }
    if let Some(ref file) = self.file {
      writer.write_tag(4);
      writer.write_string(file);
    }
    if let Some(ref span) = self.span {
      writer.write_tag(5);
      for v in span {
        writer.write_i32(*v);
      }
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// A generated file (message on wire).
#[derive(Debug, Clone, Default)]
pub struct GeneratedFile {
  pub name: Option<String>,            // tag 1
  pub insertion_point: Option<String>, // tag 2
  pub content: Option<String>,         // tag 3
                                       // tag 4: generated_code_info (SourceCodeInfo) — omitted for scaffold
}

impl GeneratedFile {
  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref name) = self.name {
      writer.write_tag(1);
      writer.write_string(name);
    }
    if let Some(ref ip) = self.insertion_point {
      writer.write_tag(2);
      writer.write_string(ip);
    }
    if let Some(ref content) = self.content {
      writer.write_tag(3);
      writer.write_string(content);
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}

/// Output from a code generator plugin (message on wire).
#[derive(Debug, Clone, Default)]
pub struct CodeGeneratorResponse {
  pub error: Option<String>,                // tag 1
  pub files: Option<Vec<GeneratedFile>>,    // tag 2
  pub diagnostics: Option<Vec<Diagnostic>>, // tag 3
}

impl CodeGeneratorResponse {
  pub fn encode(&self, writer: &mut BebopWriter) {
    let pos = writer.reserve_message_length();
    if let Some(ref error) = self.error {
      writer.write_tag(1);
      writer.write_string(error);
    }
    if let Some(ref files) = self.files {
      writer.write_tag(2);
      writer.write_array(files, |w, f| f.encode(w));
    }
    if let Some(ref diags) = self.diagnostics {
      writer.write_tag(3);
      writer.write_array(diags, |w, d| d.encode(w));
    }
    writer.write_end_marker();
    writer.fill_message_length(pos);
  }
}
