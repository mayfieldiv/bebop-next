// Types generated from bebop/schemas/bebop/{descriptor,plugin}.bop
// Regenerate with: ./generate.sh

#![allow(dead_code, unused_imports)]

use bebop_runtime::{BebopReader, BebopWriter, DecodeError};
use std::collections::HashMap;

include!("descriptor.bb.rs");
include!("plugin.bb.rs");

// ── Hand-written extensions ────────────────────────────────────

impl DefinitionKind {
  pub fn name(self) -> &'static str {
    match self {
      Self::ENUM => "enum",
      Self::STRUCT => "struct",
      Self::MESSAGE => "message",
      Self::UNION => "union",
      Self::SERVICE => "service",
      Self::CONST => "const",
      Self::DECORATOR => "decorator",
      _ => "unknown",
    }
  }
}

impl TypeKind {
  pub fn is_scalar(self) -> bool {
    self.0 >= 1 && self.0 <= 19
  }
}

impl std::fmt::Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
    if !self.suffix.is_empty() {
      write!(f, "-{}", self.suffix)?;
    }
    Ok(())
  }
}
