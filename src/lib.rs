use chumsky::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;
use wasm_bindgen::prelude::*;

use eigen_output::generate_eigen_output;
use error::TranspilerError;
use ml_parser::parser;
use transform::transform_ast;
use type_inference::parse_type;

mod eigen_output;
mod error;
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

#[wasm_bindgen]
pub struct TranspilerOutput {
    result: String,
    warnings: String,
}

#[wasm_bindgen]
impl TranspilerOutput {
    #[wasm_bindgen(getter)]
    pub fn result(&self) -> String {
        self.result.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> String {
        self.warnings.clone()
    }
}

#[wasm_bindgen]
pub fn transpile_wrap(src: &str, types: &str) -> TranspilerOutput {
    let mut warnings = String::new();

    transpile(src, types, &mut warnings)
        .map(|r| TranspilerOutput {
            result: r,
            warnings: warnings,
        })
        .unwrap_or_else(|e| TranspilerOutput {
            result: String::new(),
            warnings: format!("{}", e.0),
        })
}

fn transpile(src: &str, types: &str, warnings: &mut String) -> Result<String, TranspilerError> {
    let mut ti_state = HashMap::new();
    for line in types.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with("#") {
            match parse_type(line) {
                Ok((name, matrix_type)) => {
                    ti_state.insert(name.to_string(), matrix_type);
                }
                Err(e) => {
                    writeln!(warnings, "Error parsing <{}>: {}", line, e.0).unwrap();
                }
            }
        }
    }

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            let ast = transform_ast(ast);
            Ok(generate_eigen_output(ast, &mut ti_state, warnings))
        }
        None => Err(TranspilerError(format!("Error while parsing. {:#?}", err))),
    }
}
