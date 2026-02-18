use super::{DefinitionDescriptor, Edition, SourceCodeInfo};
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Descriptor for a single .bop source file (message on wire).
///
/// Definitions are topologically sorted: every type appears after
/// the types it depends on.
#[derive(Debug, Clone, Default)]
pub struct SchemaDescriptor {
  pub path: Option<String>,                           // tag 1
  pub package: Option<String>,                        // tag 2
  pub edition: Option<Edition>,                       // tag 3
  pub imports: Option<Vec<String>>,                   // tag 4
  pub definitions: Option<Vec<DefinitionDescriptor>>, // tag 5
  pub source_code_info: Option<SourceCodeInfo>,       // tag 6
}

impl SchemaDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut s = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => s.path = Some(reader.read_string()?),
        2 => s.package = Some(reader.read_string()?),
        3 => s.edition = Some(Edition::decode(reader)?),
        4 => s.imports = Some(reader.read_array(|r| r.read_string())?),
        5 => s.definitions = Some(reader.read_array(DefinitionDescriptor::decode)?),
        6 => s.source_code_info = Some(SourceCodeInfo::decode(reader)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(s)
  }
}
