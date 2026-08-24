use chumsky::prelude::*;
use eigen_output::generate_eigen_output;
use ml_parser::parser;
use std::collections::HashMap;
use transform::transform_ast;
use type_inference::name_to_type;
use wasm_bindgen::prelude::*;

mod eigen_output;
mod ml_parser;
mod syntax;
mod transform;
mod type_inference;

#[wasm_bindgen]
pub fn transpile(src: &str, types: &str) -> String {
    let mut ti_state = HashMap::new();
    for line in types.lines() {
        let (first, second) = line.split_once(": ").unwrap();
        ti_state.insert(first.to_string(), name_to_type(second));
    }

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            let ast = transform_ast(ast);
            generate_eigen_output(ast, &mut ti_state)
        }
        None => {
            format!("Error while parsing. {:#?}", err)
        }
    }
}
