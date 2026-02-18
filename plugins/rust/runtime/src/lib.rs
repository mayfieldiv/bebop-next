mod error;
mod reader;
mod writer;

pub use error::DecodeError;
pub use reader::BebopReader;
pub use writer::BebopWriter;

/// IEEE 754 binary16 half-precision float, stored as raw bits.
///
/// Rust has no stable `f16` type yet. This newtype wraps the raw `u16`
/// representation so generated code compiles against a concrete type.
/// Convert to `f32` for arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct F16(pub u16);

/// bfloat16 (Brain Floating Point), stored as raw bits.
///
/// This is the upper 16 bits of an IEEE 754 `f32`. This newtype wraps
/// the raw `u16` representation. Convert to `f32` for arithmetic by
/// shifting left 16 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BF16(pub u16);
