use std::collections::HashMap;

use super::{DecoratorTarget, LiteralKind, LiteralValue, TypeKind};
use crate::error::DecodeError;
use crate::wire::BebopReader;

/// Named or positional argument in a decorator usage (struct on wire).
///
/// Struct encoding: fields in order, no tags, no length prefix.
#[derive(Debug, Clone)]
pub struct DecoratorArg {
  pub name: String,
  pub value: LiteralValue,
}

impl DecoratorArg {
  pub fn decode(reader: &mut BebopReader) -> Result<Self, DecodeError> {
    let name = reader.read_string()?;
    let value = LiteralValue::decode(reader)?;
    Ok(Self { name, value })
  }
}

/// Decorator applied to a definition, field, etc. (message on wire).
#[derive(Debug, Clone, Default)]
pub struct DecoratorUsage {
  pub fqn: Option<String>,                                // tag 1
  pub args: Option<Vec<DecoratorArg>>,                    // tag 2
  pub export_data: Option<HashMap<String, LiteralValue>>, // tag 3
}

impl DecoratorUsage {
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
        1 => d.fqn = Some(reader.read_string()?),
        2 => d.args = Some(reader.read_array(DecoratorArg::decode)?),
        3 => {
          d.export_data = Some(reader.read_map(|r| {
            let k = r.read_string()?;
            let v = LiteralValue::decode(r)?;
            Ok((k, v))
          })?);
        }
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Decorator parameter definition (message on wire).
#[derive(Debug, Clone, Default)]
pub struct DecoratorParamDef {
  pub name: Option<String>,                      // tag 1
  pub description: Option<String>,               // tag 2
  pub param_type: Option<TypeKind>,              // tag 3
  pub required: Option<bool>,                    // tag 4
  pub default_value: Option<LiteralValue>,       // tag 5
  pub allowed_values: Option<Vec<LiteralValue>>, // tag 6
}

impl DecoratorParamDef {
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
        1 => d.name = Some(reader.read_string()?),
        2 => d.description = Some(reader.read_string()?),
        3 => d.param_type = Some(TypeKind::decode(reader)?),
        4 => d.required = Some(reader.read_bool()?),
        5 => d.default_value = Some(LiteralValue::decode(reader)?),
        6 => d.allowed_values = Some(reader.read_array(LiteralValue::decode)?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

/// Decorator definition body (message on wire).
#[derive(Debug, Clone, Default)]
pub struct DecoratorDef {
  pub targets: Option<DecoratorTarget>,       // tag 1
  pub allow_multiple: Option<bool>,           // tag 2
  pub params: Option<Vec<DecoratorParamDef>>, // tag 3
  pub validate_source: Option<String>,        // tag 4
  pub export_source: Option<String>,          // tag 5
}

impl DecoratorDef {
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
        1 => d.targets = Some(DecoratorTarget::decode(reader)?),
        2 => d.allow_multiple = Some(reader.read_bool()?),
        3 => d.params = Some(reader.read_array(DecoratorParamDef::decode)?),
        4 => d.validate_source = Some(reader.read_string()?),
        5 => d.export_source = Some(reader.read_string()?),
        _ => {
          reader.skip(end - reader.position())?;
        }
      }
    }
    Ok(d)
  }
}

// Suppress unused import warnings for types referenced in struct signatures
// that aren't directly used in method bodies.
const _: () = {
  fn _assert_used(_: LiteralKind) {}
};
