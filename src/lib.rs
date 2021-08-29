#![doc = include_str!("../README.md")]
#![deny(missing_docs, missing_debug_implementations, nonstandard_style)]
#![warn(unreachable_pub, rust_2018_idioms)]

pub use miette_derive::*;

pub use context::*;
pub use deprecated::*;
pub use error::*;
pub use eyreish::*;
pub use named_source::*;
pub use printer::*;
pub use protocol::*;

mod chain;
mod context;
mod deprecated;
mod error;
mod eyreish;
mod named_source;
mod printer;
mod protocol;
mod source_impls;

#[cfg(doctest)]
mod compile_test;
