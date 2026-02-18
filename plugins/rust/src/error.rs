use std::fmt;

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

impl std::error::Error for DecodeError {}

#[derive(Debug)]
pub enum GeneratorError {
  Decode(DecodeError),
  Io(std::io::Error),
  EmptyInput,
  MalformedDefinition(String),
  MalformedType(String),
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
    }
  }
}

impl std::error::Error for GeneratorError {}
