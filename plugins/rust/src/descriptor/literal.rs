use super::LiteralKind;
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Concrete literal value (message on wire).
#[derive(Debug, Clone, Default)]
pub struct LiteralValue {
  pub kind: Option<LiteralKind>,           // tag 1
  pub bool_value: Option<bool>,            // tag 2
  pub int_value: Option<i64>,              // tag 3
  pub float_value: Option<f64>,            // tag 4
  pub string_value: Option<String>,        // tag 5
  pub uuid_value: Option<[u8; 16]>,        // tag 6
  pub raw_value: Option<String>,           // tag 7
  pub bytes_value: Option<Vec<u8>>,        // tag 8
  pub timestamp_value: Option<(i64, i32)>, // tag 9
  pub duration_value: Option<(i64, i32)>,  // tag 10
}

impl LiteralValue {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut v = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => v.kind = Some(LiteralKind::decode(reader)?),
        2 => v.bool_value = Some(reader.read_bool()?),
        3 => v.int_value = Some(reader.read_i64()?),
        4 => v.float_value = Some(reader.read_f64()?),
        5 => v.string_value = Some(reader.read_string()?),
        6 => v.uuid_value = Some(reader.read_uuid()?),
        7 => v.raw_value = Some(reader.read_string()?),
        8 => v.bytes_value = Some(reader.read_byte_array()?),
        9 => v.timestamp_value = Some(reader.read_timestamp()?),
        10 => v.duration_value = Some(reader.read_duration()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(v)
  }
}
