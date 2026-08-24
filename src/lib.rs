use chumsky::prelude::*;
use eigen_output::generate_eigen_output;
use error::TranspilerError;
use ml_parser::parser;
use std::collections::HashMap;
use transform::transform_ast;
use type_inference::name_to_type;
use wasm_bindgen::prelude::*;

mod eigen_output;
mod error;
mod ml_parser;
mod syntax;
mod transform;
mod type_inference;

#[wasm_bindgen]
pub fn transpile(src: &str, types: &str) -> String {
    transpile_impl(src, types).unwrap_or_else(|e| e.to_string())
}

fn transpile_impl(src: &str, types: &str) -> Result<String, TranspilerError> {
    let mut ti_state = HashMap::new();
    for line in types.lines() {
        if !line.trim().is_empty() {
            let (first, second) = line.split_once(": ").ok_or(TranspilerError(
                "Types should be written as <name: type>.".to_string(),
            ))?;
            ti_state.insert(first.to_string(), name_to_type(second)?);
        }
    }

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            let ast = transform_ast(ast);
            generate_eigen_output(ast, &mut ti_state)
        }
        None => Err(TranspilerError(format!("Error while parsing. {:#?}", err))),
    }
}
