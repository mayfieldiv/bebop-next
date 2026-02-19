#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
#[rustfmt::skip]
pub mod test_types;

#[cfg(feature = "alloc-map")]
#[rustfmt::skip]
pub mod no_std_types;
