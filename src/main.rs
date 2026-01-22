use chumsky::prelude::*;
use eigen_output::generate_output_file;
use ml_parser::parser;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Write,
};
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
            ("x_est", (13, 1)),
            ("lastZ", (15, 1)),
        ]
        .map(|(name, (rows, cols))| (name.to_string(), (rows, cols))),
    );

    let (ast, err) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            let mut file = File::create("out.dbg").unwrap();
            let _ = file.write_all(format!("{ast:#?}").as_bytes());
            let ast = transform_ast(ast);
            generate_output_file(ast, &mut ti_state);
        }
        None => {
            println!("Error while parsing. {:#?}", err);
        }
    }
}
