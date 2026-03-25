use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
  UnexpectedEof {
    needed: usize,
    available: usize,
    /// The type being decoded when the EOF occurred. Empty string if not yet set.
    type_name: &'static str,
    /// The field being decoded when the EOF occurred. Empty string if not yet set.
    field_name: &'static str,
  },
  InvalidUtf8 {
    /// The type being decoded when the invalid UTF-8 was found. Empty string if not yet set.
    type_name: &'static str,
    /// The field being decoded when the invalid UTF-8 was found. Empty string if not yet set.
    field_name: &'static str,
  },
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

impl DecodeError {
  /// Enrich a contextless error variant with type/field context.
  ///
  /// Only enriches [`DecodeError::UnexpectedEof`] and [`DecodeError::InvalidUtf8`] when
  /// they have not already been enriched (i.e., `type_name` is empty). All other
  /// variants, and already-enriched contextless variants, pass through unchanged.
  ///
  /// This means the **innermost** `.for_field()` call wins when errors propagate
  /// through nested decode calls. Outer decode frames calling `.for_field()` provide
  /// a fallback in case no inner context was set.
  pub fn with_context(self, type_name: &'static str, field_name: &'static str) -> Self {
    match self {
      DecodeError::UnexpectedEof {
        needed,
        available,
        type_name: "",
        ..
      } => DecodeError::UnexpectedEof {
        needed,
        available,
        type_name,
        field_name,
      },
      DecodeError::InvalidUtf8 { type_name: "", .. } => DecodeError::InvalidUtf8 {
        type_name,
        field_name,
      },
      other => other,
    }
  }
}

/// Extension trait for adding field context to [`DecodeError`] results.
///
/// Provides the `.for_field(type_name, field_name)` method on
/// `Result<T, DecodeError>`. In generated code, every field read is
/// wrapped with `.for_field("TypeName", "field_name")` so that decode
/// errors report exactly where in the schema the failure occurred.
///
/// Inspired by the [`anyhow::Context`](https://docs.rs/anyhow/latest/anyhow/trait.Context.html) pattern.
/// Only enriches [`DecodeError::UnexpectedEof`] and [`DecodeError::InvalidUtf8`]
/// (the contextless variants); all others pass through unchanged. The innermost
/// `.for_field()` call wins — outer callers add context only when none exists.
pub trait DecodeContext<T> {
  fn for_field(
    self,
    type_name: &'static str,
    field_name: &'static str,
  ) -> core::result::Result<T, DecodeError>;
}

impl<T> DecodeContext<T> for core::result::Result<T, DecodeError> {
  #[inline]
  fn for_field(
    self,
    type_name: &'static str,
    field_name: &'static str,
  ) -> core::result::Result<T, DecodeError> {
    self.map_err(|e| e.with_context(type_name, field_name))
  }
}

impl fmt::Display for DecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnexpectedEof {
        needed,
        available,
        type_name,
        field_name,
      } => {
        if type_name.is_empty() {
          write!(
            f,
            "unexpected eof: needed {} bytes, {} available",
            needed, available
          )
        } else {
          write!(
            f,
            "unexpected eof in {}.{}: needed {} bytes, {} available",
            type_name, field_name, needed, available
          )
        }
      }
      Self::InvalidUtf8 {
        type_name,
        field_name,
      } => {
        if type_name.is_empty() {
          write!(f, "invalid utf-8 in string")
        } else {
          write!(f, "invalid utf-8 in {}.{}", type_name, field_name)
        }
      }
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
