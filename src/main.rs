use chumsky::prelude::*;
use eigen_output::generate_output_file;
use ml_parser::parser;
use std::{collections::HashMap, env, fs};
use transform::transform_ast;

mod eigen_output;
mod ml_parser;
mod syntax;
mod transform;
mod type_inference;

fn main() {
    let src = fs::read_to_string(env::args().nth(1).expect("Expected file argument"))
        .expect("Failed to read file");

    // type_inference state - stores function return types and matrix state
    let mut ti_state = HashMap::from(
        [
            ("_self", (13, 1)), // return type of the function being converted
            ("quatRot", (3, 3)),
            ("StateTransitionMat", (12, 12)),
            ("HamiltonianProd", (4, 4)),
            ("expm", (12, 12)), // matrixExpPade6
            ("GND", (1, 1)),
            ("dT", (1, 1)),
            ("constantsASTRA.g", (1, 1)),
            ("constantsASTRA.Q", (12, 12)),
            ("constantsASTRA.R", (6, 6)),
            ("constantsASTRA.mag", (3, 1)),
            ("P", (12, 12)),
            ("z", (15, 1)),
            ("x_est", (13, 1)),
        ]
        .map(|(name, (rows, cols))| (name.to_string(), (rows, cols))),
    );

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            println!("{ast:#?}");
            let ast = transform_ast(ast);
            generate_output_file(ast, &mut ti_state);
        }
        None => {
            println!("Error while parsing. {:#?}", err);
        }
    }
}
