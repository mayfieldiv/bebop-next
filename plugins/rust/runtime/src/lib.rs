#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;
mod reader;
mod traits;
pub mod wire_size;
mod writer;

pub use error::DecodeError;
pub use half::bf16;
pub use half::f16;
#[cfg(not(feature = "std"))]
pub use hashbrown::HashMap;
pub use reader::BebopReader;
#[cfg(feature = "std")]
pub use std::collections::HashMap;
pub use traits::{BebopDecode, BebopDecodeOwned};
pub use traits::{BebopEncode, BebopFlagBits, BebopFlags, FixedScalar};
pub use writer::BebopWriter;
