// Types generated from bebop/schemas/bebop/{descriptor,plugin}.bop
// Regenerate with: ./generate.sh

#[rustfmt::skip]
mod descriptor;
#[rustfmt::skip]
mod plugin;

pub use descriptor::*;
pub use plugin::*;

// ── Hand-written extensions ────────────────────────────────────

impl DefinitionKind {
  pub fn name(self) -> &'static str {
    match self {
      Self::Unknown => "unknown",
      Self::Enum => "enum",
      Self::Struct => "struct",
      Self::Message => "message",
      Self::Union => "union",
      Self::Service => "service",
      Self::Const => "const",
      Self::Decorator => "decorator",
    }
  }
}

impl TypeKind {
  pub fn is_scalar(self) -> bool {
    matches!(
      self,
      Self::Bool
        | Self::Byte
        | Self::Int8
        | Self::Int16
        | Self::Uint16
        | Self::Int32
        | Self::Uint32
        | Self::Int64
        | Self::Uint64
        | Self::Int128
        | Self::Uint128
        | Self::Float16
        | Self::Float32
        | Self::Float64
        | Self::Bfloat16
        | Self::String
        | Self::Uuid
        | Self::Timestamp
        | Self::Duration
    )
  }
}

impl std::fmt::Display for Version<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
    if !self.suffix.is_empty() {
      write!(f, "-{}", self.suffix)?;
    }
    Ok(())
  }
}
