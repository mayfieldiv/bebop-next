use super::*;
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Enum definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct EnumDef {
  pub base_type: Option<TypeKind>,                // tag 1
  pub members: Option<Vec<EnumMemberDescriptor>>, // tag 2
  pub is_flags: Option<bool>,                     // tag 3
}

impl EnumDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.base_type = Some(TypeKind::decode(reader)?),
        2 => d.members = Some(reader.read_array(EnumMemberDescriptor::decode)?),
        3 => d.is_flags = Some(reader.read_bool()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Struct definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct StructDef {
  pub fields: Option<Vec<FieldDescriptor>>, // tag 1
  pub is_mutable: Option<bool>,             // tag 2
  pub fixed_size: Option<u32>,              // tag 3
}

impl StructDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.fields = Some(reader.read_array(FieldDescriptor::decode)?),
        2 => d.is_mutable = Some(reader.read_bool()?),
        3 => d.fixed_size = Some(reader.read_u32()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Message definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct MessageDef {
  pub fields: Option<Vec<FieldDescriptor>>, // tag 1
}

impl MessageDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.fields = Some(reader.read_array(FieldDescriptor::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Union definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct UnionDef {
  pub branches: Option<Vec<UnionBranchDescriptor>>, // tag 1
}

impl UnionDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.branches = Some(reader.read_array(UnionBranchDescriptor::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Service definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct ServiceDef {
  pub methods: Option<Vec<MethodDescriptor>>, // tag 1
}

impl ServiceDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.methods = Some(reader.read_array(MethodDescriptor::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Const definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct ConstDef {
  pub const_type: Option<TypeDescriptor>, // tag 1
  pub value: Option<LiteralValue>,        // tag 2
}

impl ConstDef {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.const_type = Some(TypeDescriptor::decode(reader)?),
        2 => d.value = Some(LiteralValue::decode(reader)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Named definition (message on wire).
///
/// The `kind` field selects which body field is populated. Exactly one
/// body is present per definition.
#[derive(Debug, Clone, Default)]
pub struct DefinitionDescriptor {
  pub kind: Option<DefinitionKind>,              // tag 1
  pub name: Option<String>,                      // tag 2
  pub fqn: Option<String>,                       // tag 3
  pub documentation: Option<String>,             // tag 4
  pub visibility: Option<Visibility>,            // tag 5
  pub decorators: Option<Vec<DecoratorUsage>>,   // tag 6
  pub nested: Option<Vec<DefinitionDescriptor>>, // tag 7
  pub enum_def: Option<EnumDef>,                 // tag 8
  pub struct_def: Option<StructDef>,             // tag 9
  pub message_def: Option<MessageDef>,           // tag 10
  pub union_def: Option<UnionDef>,               // tag 11
  pub service_def: Option<ServiceDef>,           // tag 12
  pub const_def: Option<ConstDef>,               // tag 13
  pub decorator_def: Option<DecoratorDef>,       // tag 14
}

impl DefinitionDescriptor {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let length = reader.read_message_length()? as usize;
    let end = reader.position() + length;
    let mut d = Self::default();

    while reader.position() < end {
      let tag = reader.read_tag()?;
      if tag == 0 {
        break;
      }
      match tag {
        1 => d.kind = Some(DefinitionKind::decode(reader)?),
        2 => d.name = Some(reader.read_string()?),
        3 => d.fqn = Some(reader.read_string()?),
        4 => d.documentation = Some(reader.read_string()?),
        5 => d.visibility = Some(Visibility::decode(reader)?),
        6 => d.decorators = Some(reader.read_array(DecoratorUsage::decode)?),
        7 => d.nested = Some(reader.read_array(DefinitionDescriptor::decode)?),
        8 => d.enum_def = Some(EnumDef::decode(reader)?),
        9 => d.struct_def = Some(StructDef::decode(reader)?),
        10 => d.message_def = Some(MessageDef::decode(reader)?),
        11 => d.union_def = Some(UnionDef::decode(reader)?),
        12 => d.service_def = Some(ServiceDef::decode(reader)?),
        13 => d.const_def = Some(ConstDef::decode(reader)?),
        14 => d.decorator_def = Some(DecoratorDef::decode(reader)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}
