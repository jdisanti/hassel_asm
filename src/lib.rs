//
// Copyright 2017 hassel_asm Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate hassel_lib6502;

mod assembler;
pub mod ast;
pub mod error;
pub mod ir;
pub mod src_tag;
mod src_unit;

pub use assembler::{Assembler, AssemblerOutput};
