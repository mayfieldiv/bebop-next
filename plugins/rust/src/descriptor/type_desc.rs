use super::TypeKind;
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Type reference (message on wire).
///
/// The `kind` field determines which other fields are present:
/// - Scalars: no additional fields
/// - ARRAY: `array_element`
/// - FIXED_ARRAY: `fixed_array_element` + `fixed_array_size`
/// - MAP: `map_key` + `map_value`
/// - DEFINED: `defined_fqn`
#[derive(Debug, Clone, Default)]
pub struct TypeDescriptor {
  pub kind: Option<TypeKind>,                           // tag 1
  pub array_element: Option<Box<TypeDescriptor>>,       // tag 2
  pub fixed_array_element: Option<Box<TypeDescriptor>>, // tag 3
  pub fixed_array_size: Option<u32>,                    // tag 4
  pub map_key: Option<Box<TypeDescriptor>>,             // tag 5
  pub map_value: Option<Box<TypeDescriptor>>,           // tag 6
  pub defined_fqn: Option<String>,                      // tag 7
}

impl TypeDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut td = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => td.kind = Some(TypeKind::decode(reader)?),
        2 => td.array_element = Some(Box::new(TypeDescriptor::decode(reader)?)),
        3 => td.fixed_array_element = Some(Box::new(TypeDescriptor::decode(reader)?)),
        4 => td.fixed_array_size = Some(reader.read_u32()?),
        5 => td.map_key = Some(Box::new(TypeDescriptor::decode(reader)?)),
        6 => td.map_value = Some(Box::new(TypeDescriptor::decode(reader)?)),
        7 => td.defined_fqn = Some(reader.read_string()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(td)
  }
}
