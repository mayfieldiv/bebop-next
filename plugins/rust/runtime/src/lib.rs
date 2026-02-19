#![no_std]

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "alloc")]
extern crate alloc;

mod error;
mod reader;
mod traits;
pub mod wire_size;
#[cfg(feature = "alloc")]
mod writer;

pub use error::DecodeError;
pub use reader::BebopReader;
pub use traits::{BebopDecode, BebopDecodeOwned};
#[cfg(feature = "alloc")]
pub use traits::{BebopEncode, BebopFlagBits, BebopFlags};
#[cfg(feature = "alloc")]
pub use writer::BebopWriter;
pub use half::bf16;
pub use half::f16;
