error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        FmtError(::std::fmt::Error);
    }

    errors {
        ParseError(errors: Vec<String>) {
            description("Failed to parse assembly")
            display("Failed to parse assembly:\n{}", errors.join("\n"))
        }
    }
}