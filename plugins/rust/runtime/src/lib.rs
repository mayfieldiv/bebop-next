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
pub use error::DecodeError;
pub use half::bf16;
pub use half::f16;
#[cfg(not(feature = "std"))]
pub use hashbrown::HashMap;
pub use reader::BebopReader;
#[cfg(feature = "serde")]
pub use serde;
#[cfg(feature = "serde")]
pub use serde_bytes;
#[cfg(feature = "std")]
pub use std::collections::HashMap;
pub use temporal::{BebopDuration, BebopTimestamp};
pub use traits::{BebopDecode, BebopDecodeOwned};
pub use traits::{BebopEncode, BebopFlagBits, BebopFlags, FixedScalar};
pub use uuid::Uuid;
pub use writer::BebopWriter;
