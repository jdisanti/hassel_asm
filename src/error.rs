//
// Copyright 2017 hassel_asm Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use src_tag::SrcTag;
use src_unit::SrcUnits;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        IoError(::std::io::Error);
        FmtError(::std::fmt::Error);
        JsonError(::serde_json::Error);
    }

    errors {
        ParseError(errors: Vec<String>) {
            description("Failed to parse assembly")
            display("Failed to parse assembly:\n{}", errors.join("\n"))
        }
        AssemblerError(src_tag: SrcTag, msg: String) {
            description("Failed to assemble")
            display("{}", msg)
        }
        SrcUnitError(msg: String) {
            description("Failed to assemble")
            display("{}", msg)
        }
    }
}

pub fn format_error(src_units: &SrcUnits, error: &Error) -> String {
    match error.0 {
        ErrorKind::AssemblerError(ref src_tag, ref msg) => {
            let row_col = src_tag.row_col(src_units.source(src_tag.unit));
            format!(
                "{}:{}:{}: {}",
                src_units.name(src_tag.unit).clone(),
                row_col.0,
                row_col.1,
                msg
            )
        }
        _ => format!("{}", error),
    }
}
