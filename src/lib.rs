use chumsky::prelude::*;
use eigen_output::generate_eigen_output;
use ml_parser::parser;
use std::collections::HashMap;
use transform::transform_ast;
use wasm_bindgen::prelude::*;

mod eigen_output;
mod ml_parser;
mod syntax;
mod transform;
mod type_inference;

#[wasm_bindgen]
pub fn transpile(src: &str) -> String {
    // TODO - import TI state

    // type_inference state - stores function return types and matrix state
    let mut ti_state = HashMap::from(
        [
            ("_self", (13, 1)), // return type of the function being converted
            ("M_PI", (1, 1)),
            // used across several functions
            ("constantsASTRA.g", (1, 1)),
            ("constantsASTRA.m", (1, 1)),
            ("constantsASTRA.Q", (18, 18)),
            ("constantsASTRA.R", (6, 6)),
            ("constantsASTRA.mag", (3, 1)),
            // pablo's functions
            ("StateTransitionMat", (9, 9)),
            ("HamiltonianProd", (4, 4)),
            ("zetaCross", (3, 3)),
            ("quatRot", (3, 3)),
            // estimator types
            ("dT", (1, 1)),
            ("P", (9, 9)),
            ("P0", (9, 9)),
            ("z", (15, 1)),
            ("x_est", (19, 1)),
            ("lastZ", (15, 1)),
        ]
        .map(|(name, (rows, cols))| (name.to_string(), (rows, cols))),
    );

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
