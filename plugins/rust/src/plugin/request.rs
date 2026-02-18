use std::collections::HashMap;

use crate::descriptor::SchemaDescriptor;
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Compiler version (struct on wire — positional, no tags).
#[derive(Debug, Clone)]
pub struct Version {
  pub major: i32,
  pub minor: i32,
  pub patch: i32,
  pub suffix: String,
}

impl Version {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    Ok(Self {
      major: reader.read_i32()?,
      minor: reader.read_i32()?,
      patch: reader.read_i32()?,
      suffix: reader.read_string()?,
    })
  }
}

impl std::fmt::Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
    if !self.suffix.is_empty() {
      write!(f, "-{}", self.suffix)?;
    }
    Ok(())
  }
}

/// Input to a code generator plugin (message on wire).
#[derive(Debug, Clone, Default)]
pub struct CodeGeneratorRequest {
  pub files_to_generate: Option<Vec<String>>,        // tag 1
  pub parameter: Option<String>,                     // tag 2
  pub compiler_version: Option<Version>,             // tag 3
  pub schemas: Option<Vec<SchemaDescriptor>>,        // tag 4
  pub host_options: Option<HashMap<String, String>>, // tag 5
}

impl CodeGeneratorRequest {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut req = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => req.files_to_generate = Some(reader.read_array(|r| r.read_string())?),
        2 => req.parameter = Some(reader.read_string()?),
        3 => req.compiler_version = Some(Version::decode(reader)?),
        4 => req.schemas = Some(reader.read_array(SchemaDescriptor::decode)?),
        5 => {
          req.host_options = Some(reader.read_map(|r| {
            let k = r.read_string()?;
            let v = r.read_string()?;
            Ok((k, v))
          })?);
        }
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(req)
  }
}
