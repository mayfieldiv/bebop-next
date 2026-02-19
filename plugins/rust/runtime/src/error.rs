use core::fmt;

#[derive(Debug)]
pub enum DecodeError {
  UnexpectedEof { needed: usize, available: usize },
  InvalidUtf8,
  InvalidEnum { type_name: &'static str, value: u64 },
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
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}
