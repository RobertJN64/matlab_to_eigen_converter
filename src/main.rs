use chumsky::prelude::*;
use eigen_output::generate_eigen_output;
use ml_parser::parser;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Write,
};
use transform::transform_ast;
use type_inference::parse_type;

mod eigen_output;
mod error;
mod mex_output;
mod ml_parser;
mod syntax;
mod transform;
mod type_inference;

// Error handling notes:
// Because transpile_wrap() is called from WASM it must never panic or output to stdout or stderr.
// 2 output strings are provided instead: the result (C++ code) and warnings, which should contain
// any errors encountered during the transpiling process. Unrecoverable errors can be returned as a
// Result<_, TranspilerError> which will be sent to the warnings output. This should be avoided where
// possible, as the transpiler will then not emit C++ output and is difficult to debug. Warnings should
// include the C++ line number where possible.

// TODO - replace linenum system with source line number
// TODO - list of types used at the top

fn main() {
    let src = fs::read_to_string(env::args().nth(1).expect("Expected file argument"))
        .expect("Failed to read file");
    let types = fs::read_to_string(env::args().nth(2).expect("Expected two file arguments"))
        .expect("Failed to read file");

    let mut ti_state = HashMap::new();
    for line in types.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with("#") {
            match parse_type(line) {
                Ok((name, matrix_type)) => {
                    ti_state.insert(name.to_string(), matrix_type);
                }
                Err(e) => {
                    println!("Error parsing <{}>: {}", line, e.0);
                }
            }
        }
    }

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            let mut dbg_file = File::create("out.dbg").unwrap();
            let _ = dbg_file.write_all(format!("{ast:#?}").as_bytes());
            let ast = transform_ast(ast);
            let mut cpp_file = File::create("out.cpp").unwrap();
            let mut warnings = String::new();
            let _ = cpp_file.write_all(
                generate_eigen_output(ast, &mut ti_state, &mut warnings, true).as_bytes(),
            );
        }
        None => println!("Error while parsing. {:#?}", err),
    }
}
