#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod bytes;
mod error;
mod reader;
mod temporal;
mod traits;
pub mod wire_size;
mod writer;

pub use bytes::BebopBytes;
pub use error::{DecodeContext, DecodeError};
#[cfg(feature = "half")]
pub use half::bf16;
#[cfg(feature = "half")]
pub use half::f16;
pub use hashbrown::HashMap;
pub use reader::BebopReader;
#[cfg(feature = "serde")]
pub use serde;
pub use temporal::{BebopDuration, BebopTimestamp};
pub use traits::{BebopDecode, BebopDecodeOwned};
pub use traits::{BebopEncode, BebopFlagBits, BebopFlags, FixedScalar};
#[cfg(feature = "uuid")]
pub use uuid::Uuid;
pub use writer::BebopWriter;
