use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Source location for a descriptor element (message on wire).
#[derive(Debug, Clone, Default)]
pub struct Location {
  pub path: Option<Vec<i32>>,                 // tag 1
  pub span: Option<[i32; 4]>,                 // tag 2
  pub leading_comments: Option<String>,       // tag 3
  pub trailing_comments: Option<String>,      // tag 4
  pub detached_comments: Option<Vec<String>>, // tag 5
}

impl Location {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut loc = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => loc.path = Some(reader.read_array(|r| r.read_i32())?),
        2 => loc.span = Some(reader.read_fixed_i32_array::<4>()?),
        3 => loc.leading_comments = Some(reader.read_string()?),
        4 => loc.trailing_comments = Some(reader.read_string()?),
        5 => loc.detached_comments = Some(reader.read_array(|r| r.read_string())?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(loc)
  }
}

/// Source code info for a schema (message on wire).
#[derive(Debug, Clone, Default)]
pub struct SourceCodeInfo {
  pub locations: Option<Vec<Location>>, // tag 1
}

impl SourceCodeInfo {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut info = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => info.locations = Some(reader.read_array(Location::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(info)
  }
}
