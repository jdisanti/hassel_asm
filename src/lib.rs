#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

extern crate hassel_lib6502;

mod assembler;
pub mod ast;
pub mod error;
pub mod ir;
pub mod src_tag;
mod src_unit;

pub use assembler::{Assembler, AssemblerOutput};
