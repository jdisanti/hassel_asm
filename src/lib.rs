#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

extern crate hassel_lib6502;

mod assembler;
mod ast;
pub mod error;
mod ir;
mod src_tag;
mod src_unit;

pub use assembler::{Assembler, AssemblerOutput};
