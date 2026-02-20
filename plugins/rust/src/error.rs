use std::fmt;

pub use bebop_runtime::DecodeError;

#[derive(Debug)]
pub enum GeneratorError {
  Decode(DecodeError),
  Io(std::io::Error),
  EmptyInput,
  MalformedDefinition(String),
  MalformedType(String),
  InvalidOption(String),
}

impl From<DecodeError> for GeneratorError {
  fn from(e: DecodeError) -> Self {
    Self::Decode(e)
  }
}

impl From<std::io::Error> for GeneratorError {
  fn from(e: std::io::Error) -> Self {
    Self::Io(e)
  }
}

impl fmt::Display for GeneratorError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Decode(e) => write!(f, "decode error: {}", e),
      Self::Io(e) => write!(f, "io error: {}", e),
      Self::EmptyInput => write!(f, "empty input on stdin"),
      Self::MalformedDefinition(msg) => write!(f, "malformed definition: {}", msg),
      Self::MalformedType(msg) => write!(f, "malformed type: {}", msg),
      Self::InvalidOption(msg) => write!(f, "invalid option: {}", msg),
    }
  }
}

impl std::error::Error for GeneratorError {}
