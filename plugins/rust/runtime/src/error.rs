use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
  UnexpectedEof {
    needed: usize,
    available: usize,
  },
  InvalidUtf8,
  InvalidEnum {
    type_name: &'static str,
    value: u64,
  },
  InvalidUnion {
    type_name: &'static str,
    discriminator: u8,
  },
  InvalidField {
    type_name: &'static str,
    tag: u8,
  },
  InvalidFlags {
    type_name: &'static str,
    bits: u64,
  },
  AllocationFailed {
    requested: usize,
  },
}

impl fmt::Display for DecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnexpectedEof { needed, available } => {
        write!(
          f,
          "unexpected eof: needed {} bytes, {} available",
          needed, available
        )
      }
      Self::InvalidUtf8 => write!(f, "invalid utf-8 in string"),
      Self::InvalidEnum { type_name, value } => {
        write!(f, "invalid {} value: {}", type_name, value)
      }
      Self::InvalidUnion {
        type_name,
        discriminator,
      } => {
        write!(f, "invalid {} discriminator: {}", type_name, discriminator)
      }
      Self::InvalidField { type_name, tag } => {
        write!(f, "invalid {} field tag: {}", type_name, tag)
      }
      Self::InvalidFlags { type_name, bits } => {
        write!(f, "invalid {} bits: {:#x}", type_name, bits)
      }
      Self::AllocationFailed { requested } => {
        write!(f, "allocation failed for {} elements", requested)
      }
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}
