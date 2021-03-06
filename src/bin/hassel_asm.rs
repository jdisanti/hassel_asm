//
// Copyright 2017 hassel_asm Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

extern crate clap;
extern crate hassel_asm;

use hassel_asm::{error, Assembler};

use std::fs::File;
use std::io::prelude::*;
use std::process;

struct Options {
    input_name: String,
    output_name: Option<String>,
}

fn die(err: &error::Error) -> ! {
    println!("{}", err.0);
    process::exit(1);
}

fn handle_result<T>(result: error::Result<T>) -> T {
    match result {
        Ok(t) => t,
        Err(err) => die(&err),
    }
}

fn get_options() -> Options {
    let cli_app = clap::App::new("hassel_asm")
        .version("v0.1.0")
        .author("John DiSanti <johndisanti@gmail.com>")
        .about("6502 Assembler")
        .arg(
            clap::Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .help("Sets output file name; otherwise outputs to STDOUT")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("INPUT")
                .help("Input source file to use")
                .required(true),
        );
    let cli_matches = cli_app.get_matches();

    Options {
        input_name: cli_matches.value_of("INPUT").unwrap().into(),
        output_name: cli_matches.value_of("OUTPUT").map(String::from),
    }
}

pub fn main() {
    let options = get_options();

    let input_source = {
        let mut file = match File::open(&options.input_name) {
            Ok(file) => file,
            Err(e) => {
                println!("Failed to open input source file: {}", e);
                return;
            }
        };
        let mut file_contents = String::new();
        if !file.read_to_string(&mut file_contents).is_ok() {
            println!("Failed to read the input source file");
            return;
        }
        file_contents
    };

    let mut assembler = Assembler::new();
    handle_result(assembler.parse_unit(&options.input_name, &input_source));

    let assembler_output = handle_result(assembler.assemble());

    let output_file_name = options.output_name.unwrap_or_else(|| "out.rom".into());
    let source_map_file_name = format!("{}.map", output_file_name);

    save_bytes(&output_file_name, &assembler_output.bytes.unwrap());
    save_bytes(
        &source_map_file_name,
        &assembler_output.source_map.unwrap().as_bytes(),
    );
}

fn save_bytes(file_name: &str, bytes: &[u8]) {
    let mut file = match File::create(file_name) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create output file: {}", e);
            process::exit(1);
        }
    };
    if !file.write_all(bytes).is_ok() {
        println!("Failed to write to output file");
        process::exit(1);
    }
}
