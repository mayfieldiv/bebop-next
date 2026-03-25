use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum DecodeError {
  UnexpectedEof {
    needed: usize,
    available: usize,
    /// The type being decoded when the EOF occurred.
    type_name: Option<&'static str>,
    /// The field being decoded when the EOF occurred.
    field_name: Option<&'static str>,
  },
  InvalidUtf8 {
    /// The type being decoded when the invalid UTF-8 was found.
    type_name: Option<&'static str>,
    /// The field being decoded when the invalid UTF-8 was found.
    field_name: Option<&'static str>,
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
  /// they have not already been enriched (i.e., `type_name` is `None`). All other
  /// variants, and already-enriched contextless variants, pass through unchanged.
  ///
  /// This means the **innermost** `.for_field()` call wins when errors propagate
  /// through nested decode calls.
  pub fn with_context(self, type_name: &'static str, field_name: &'static str) -> Self {
    debug_assert!(
      !type_name.is_empty() && !field_name.is_empty(),
      "with_context: both type_name and field_name must be non-empty"
    );
    match self {
      DecodeError::UnexpectedEof {
        needed,
        available,
        type_name: None,
        ..
      } => DecodeError::UnexpectedEof {
        needed,
        available,
        type_name: Some(type_name),
        field_name: Some(field_name),
      },
      DecodeError::InvalidUtf8 {
        type_name: None, ..
      } => DecodeError::InvalidUtf8 {
        type_name: Some(type_name),
        field_name: Some(field_name),
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
        type_name: Some(t),
        field_name: Some(fld),
      } => {
        write!(
          f,
          "unexpected eof in {}.{}: needed {} bytes, {} available",
          t, fld, needed, available
        )
      }
      Self::UnexpectedEof {
        needed, available, ..
      } => {
        write!(
          f,
          "unexpected eof: needed {} bytes, {} available",
          needed, available
        )
      }
      Self::InvalidUtf8 {
        type_name: Some(t),
        field_name: Some(fld),
      } => write!(f, "invalid utf-8 in {}.{}", t, fld),
      Self::InvalidUtf8 { .. } => write!(f, "invalid utf-8 in string"),
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
