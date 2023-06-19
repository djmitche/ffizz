#![warn(unsafe_op_in_unsafe_fn)]
#![allow(unused_unsafe)]
#![doc = include_str!("crate-doc.md")]

mod boxed;
mod unboxed;
mod util;
mod value;

pub use boxed::*;
pub use unboxed::*;
pub use value::*;
