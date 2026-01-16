use chumsky::prelude::*;
use eigen_output::generate_output_file;
use ml_parser::parser;
use std::{env, fs};

mod eigen_output;
mod ml_parser;
mod syntax;

fn main() {
    let src = fs::read_to_string(env::args().nth(1).expect("Expected file argument"))
        .expect("Failed to read file");

    let (ast, _) = parser().parse(src.trim()).into_output_errors();
    match ast {
        Some(ast) => {
            println!("{ast:#?}");
            generate_output_file(ast);
        }
        None => {
            println!("Error while parsing.");
        }
    }
}
