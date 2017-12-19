use src_tag::SrcTag;
use src_unit::SrcUnits;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
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
