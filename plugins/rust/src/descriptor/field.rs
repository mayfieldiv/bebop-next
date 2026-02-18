use super::{DecoratorUsage, MethodType, TypeDescriptor};
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Field in a struct or message (message on wire).
#[derive(Debug, Clone, Default)]
pub struct FieldDescriptor {
  pub name: Option<String>,                    // tag 1
  pub documentation: Option<String>,           // tag 2
  pub field_type: Option<TypeDescriptor>,      // tag 3
  pub index: Option<u32>,                      // tag 4
  pub decorators: Option<Vec<DecoratorUsage>>, // tag 5
}

impl FieldDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut f = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => f.name = Some(reader.read_string()?),
        2 => f.documentation = Some(reader.read_string()?),
        3 => f.field_type = Some(TypeDescriptor::decode(reader)?),
        4 => f.index = Some(reader.read_u32()?),
        5 => f.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(f)
  }
}

/// Enum member (message on wire).
#[derive(Debug, Clone, Default)]
pub struct EnumMemberDescriptor {
  pub name: Option<String>,                    // tag 1
  pub documentation: Option<String>,           // tag 2
  pub value: Option<u64>,                      // tag 3
  pub decorators: Option<Vec<DecoratorUsage>>, // tag 4
  pub value_expr: Option<String>,              // tag 5
}

impl EnumMemberDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut m = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => m.name = Some(reader.read_string()?),
        2 => m.documentation = Some(reader.read_string()?),
        3 => m.value = Some(reader.read_u64()?),
        4 => m.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        5 => m.value_expr = Some(reader.read_string()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(m)
  }
}

/// Union branch (message on wire).
#[derive(Debug, Clone, Default)]
pub struct UnionBranchDescriptor {
  pub discriminator: Option<u8>,               // tag 1
  pub documentation: Option<String>,           // tag 2
  pub inline_fqn: Option<String>,              // tag 3
  pub type_ref_fqn: Option<String>,            // tag 4
  pub name: Option<String>,                    // tag 5
  pub decorators: Option<Vec<DecoratorUsage>>, // tag 6
}

impl UnionBranchDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut b = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => b.discriminator = Some(reader.read_byte()?),
        2 => b.documentation = Some(reader.read_string()?),
        3 => b.inline_fqn = Some(reader.read_string()?),
        4 => b.type_ref_fqn = Some(reader.read_string()?),
        5 => b.name = Some(reader.read_string()?),
        6 => b.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(b)
  }
}

/// Service method (message on wire).
#[derive(Debug, Clone, Default)]
pub struct MethodDescriptor {
  pub name: Option<String>,                    // tag 1
  pub documentation: Option<String>,           // tag 2
  pub request_type: Option<TypeDescriptor>,    // tag 3
  pub response_type: Option<TypeDescriptor>,   // tag 4
  pub method_type: Option<MethodType>,         // tag 5
  pub id: Option<u32>,                         // tag 6
  pub decorators: Option<Vec<DecoratorUsage>>, // tag 7
}

impl MethodDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut m = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => m.name = Some(reader.read_string()?),
        2 => m.documentation = Some(reader.read_string()?),
        3 => m.request_type = Some(TypeDescriptor::decode(reader)?),
        4 => m.response_type = Some(TypeDescriptor::decode(reader)?),
        5 => m.method_type = Some(MethodType::decode(reader)?),
        6 => m.id = Some(reader.read_u32()?),
        7 => m.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(m)
  }
}
