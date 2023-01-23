#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![doc = include_str!("crate-doc.md")]

mod error;
mod fzstring;
mod macros;
mod utilfns;

pub use error::*;
pub use fzstring::{fz_string_t, FzString};
pub use macros::*;
pub use utilfns::*;
