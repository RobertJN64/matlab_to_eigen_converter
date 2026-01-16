use chumsky::{prelude::*, text::*};
use std::{env, fs};

#[derive(Clone, Debug)]
struct MLtFunction {
    return_obj: String,
    name: String,
    params: Vec<String>,
    body: Vec<MLtAssignment>,
}

#[derive(Clone, Debug)]
enum MLtLine {
    Assignment(MLtAssignment),
    Comment(String),
    Error(String),
}

#[derive(Clone, Debug)]
// matches `lvalue = expr;`
struct MLtAssignment {
    lvalue: MLtLValue,
    expr: MLtExpr,
}

#[derive(Clone, Debug)]
enum MLtLValue {
    Basic(String),             // `z`
    Segment(String, MLtRange), // `z(1:3)`
    Block(String, u32, u32),   // `z(1:3, 2:4)`
}

#[derive(Clone, Debug)]
enum MLtExpr {
    Basic(MLtLValue),                            // lvalue
    BinOp(Box<MLtExpr>, MLtBinOp, Box<MLtExpr>), // "lvalue + lvalue", or sub, mul, div
}

#[derive(Clone, Debug)]
struct MLtRange {
    start: u32,
    end: u32,
}

#[derive(Clone, Debug)]
enum MLtBinOp {
    Add,
    Sub,
    Mul,
    Div,
}

// used for keywords
fn kw<'src>(s: &'static str) -> impl Parser<'src, &'src str, ()> + Clone {
    just(s).padded().ignored()
}

fn parser<'src>() -> impl Parser<'src, &'src str, Vec<MLtLine>> {
    let mlt_range = int(10)
        .then_ignore(kw(":"))
        .then(int(10))
        .map(|(start, end): (&str, &str)| MLtRange {
            start: start.parse().expect("failed to parse output of int to int"),
            end: end.parse().expect("failed to parse output of int to int"),
        });

    let mlt_lvalue = choice((
        ident()
            .then(mlt_range.delimited_by(kw("("), kw(")")))
            .map(|(ident, pf): (&str, MLtRange)| MLtLValue::Segment(ident.to_string(), pf)),
        ident().map(|ident: &str| MLtLValue::Basic(ident.to_string())),
    ));

    let mlt_expr = recursive(|_| {
        let atom = mlt_lvalue.clone().map(MLtExpr::Basic);

        let mul_div = atom.clone().foldl(
            choice((kw("*").to(MLtBinOp::Mul), kw("/").to(MLtBinOp::Div)))
                .then(atom)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        mul_div.clone().foldl(
            choice((kw("+").to(MLtBinOp::Add), kw("-").to(MLtBinOp::Sub)))
                .then(mul_div)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        )
    });

    let mlt_assignment = mlt_lvalue
        .then_ignore(kw("="))
        .then(mlt_expr)
        .then_ignore(kw(";"))
        .map(|(lvalue, expr)| MLtAssignment { lvalue, expr });

    let mlt_line = choice((
        mlt_assignment.map(MLtLine::Assignment),
        none_of(";")
            .repeated()
            .at_least(1)
            .collect::<String>()
            .then_ignore(just(';'))
            .padded()
            .map(MLtLine::Error),
    ));

    return mlt_line.repeated().collect();
}

fn main() {
    let src = fs::read_to_string(env::args().nth(1).expect("Expected file argument"))
        .expect("Failed to read file");

    let (json, errs) = parser().parse(src.trim()).into_output_errors();
    println!("{json:#?}");
}
