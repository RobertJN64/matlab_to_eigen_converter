use crate::syntax::*;
use chumsky::{prelude::*, text::*};

// used for keywords
fn kw<'src>(s: &'static str) -> impl Parser<'src, &'src str, ()> + Clone {
    just(s).padded().ignored()
}

// detects newline as its own line type - makes output cleaner
fn kw_no_newline<'src>(s: &'static str) -> impl Parser<'src, &'src str, ()> + Clone {
    just(s).padded_by(text::inline_whitespace()).ignored()
}

// ident to string
fn sident<'src>() -> impl Parser<'src, &'src str, String> + Clone {
    ident().map(String::from)
}

pub fn parser<'src>() -> impl Parser<'src, &'src str, MLtFunction> {
    let mlt_range = int(10)
        .then_ignore(kw(":"))
        .then(int(10))
        .map(|(start, end)| MLtRange {
            start: start.parse().expect("failed to parse output of int to int"),
            end: end.parse().expect("failed to parse output of int to int"),
        });

    let mlt_matrix = choice((
        sident()
            .then(mlt_range.clone().delimited_by(kw("("), kw(")")))
            .map(|(ident, pf)| MLtMatrixAccess::MatrixSegment(ident, pf)),
        sident()
            .then(
                mlt_range
                    .clone()
                    .separated_by(just(" "))
                    .collect()
                    .delimited_by(kw("(["), kw("])")),
            )
            .map(|(ident, pf)| MLtMatrixAccess::MatrixMultiSegment(ident, pf)),
        sident()
            .then(
                mlt_range
                    .clone()
                    .then_ignore(kw(","))
                    .then(mlt_range)
                    .delimited_by(kw("("), kw(")")),
            )
            .map(|(ident, (range_1, range_2))| {
                MLtMatrixAccess::MatrixBlock(ident, range_1, range_2)
            }),
        sident().map(MLtMatrixAccess::Matrix),
    ));

    let mut mlt_lvalue = Recursive::declare();
    let mut mlt_expr = Recursive::declare();

    mlt_lvalue.define(choice((
        int(10)
            .then_ignore(just("."))
            .then(int(10))
            .map(|(int_v, float_v)| MLtLValue::Float(format!("{}.{}", int_v, float_v))),
        int(10)
            .then_ignore(just("e"))
            .then(int(10))
            .map(|(int_v, exp_v)| MLtLValue::Float(format!("{}e{}", int_v, exp_v))),
        int(10).map(String::from).map(MLtLValue::Integer),
        sident()
            .then(
                mlt_expr
                    .clone()
                    .separated_by(kw(","))
                    .collect()
                    .delimited_by(kw("("), kw(")")),
            )
            .map(|(function_name, params)| MLtLValue::FunctionCall(function_name, params)),
        sident()
            .then_ignore(kw("."))
            .then(mlt_matrix.clone())
            .map(|(struct_name, matrix)| MLtLValue::StructMatrix(struct_name, matrix)),
        mlt_expr
            .clone()
            .separated_by(kw(";"))
            .collect()
            .delimited_by(kw("["), kw("]"))
            .map(MLtLValue::InlineMatrix),
        mlt_matrix.map(MLtLValue::Matrix),
    )));

    mlt_expr.define({
        let atom = choice((
            mlt_expr
                .clone()
                .delimited_by(kw("("), kw(")"))
                .map(|e| MLtExpr::Parenthesized(Box::new(e))),
            mlt_lvalue.clone().map(MLtExpr::Basic),
        ));

        let negated_atom = choice((
            kw("-")
                .ignore_then(atom.clone())
                .map(|e| MLtExpr::Negation(Box::new(e))),
            atom,
        ));

        let transposed_atom = choice((
            negated_atom
                .clone()
                .then_ignore(kw("'"))
                .map(|e| MLtExpr::Transposed(Box::new(e))),
            negated_atom,
        ));

        let exponents = transposed_atom.clone().foldl(
            kw("^").to(MLtBinOp::Pow).then(transposed_atom).repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        let mul_div = exponents.clone().foldl(
            choice((kw("*").to(MLtBinOp::Mul), kw("/").to(MLtBinOp::Div)))
                .then(exponents)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        let add_sub = mul_div.clone().foldl(
            choice((kw("+").to(MLtBinOp::Add), kw("-").to(MLtBinOp::Sub)))
                .then(mul_div)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        let comparison = add_sub.clone().foldl(
            choice((
                kw("~=").to(MLtBinOp::NotEqualTo),
                kw("==").to(MLtBinOp::EqualTo),
                kw("<=").to(MLtBinOp::LessThanEqualTo),
                kw("<").to(MLtBinOp::LessThan),
                kw(">=").to(MLtBinOp::GreaterThanEqualTo),
                kw(">").to(MLtBinOp::GreaterThan),
            ))
            .then(add_sub)
            .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        // logical ops
        comparison.clone().foldl(
            choice((kw("&&").to(MLtBinOp::And), kw("||").to(MLtBinOp::Or)))
                .then(comparison)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        )
    });

    let mlt_assignment = mlt_lvalue
        .then_ignore(kw("="))
        .then(mlt_expr.clone())
        .then_ignore(kw_no_newline(";"));

    let mut mlt_statement = Recursive::declare();

    mlt_statement.define(choice((
        mlt_assignment.map(|(lvalue, expr)| MLtStatement::Assignment(lvalue, expr)),
        kw_no_newline("\r\n").to(MLtStatement::NewLine),
        kw_no_newline("\n").to(MLtStatement::NewLine),
        kw_no_newline("persistent")
            .ignore_then(none_of("\r\n").repeated().collect::<String>())
            .padded()
            .map(|s| MLtStatement::Persistent(s.split_whitespace().map(String::from).collect())),
        kw_no_newline("if")
            .ignore_then(mlt_expr)
            .padded_by(text::inline_whitespace())
            .then(mlt_statement.clone().repeated().collect())
            .then_ignore(kw_no_newline("end"))
            .map(|(cond, body)| MLtStatement::IfStatement(cond, body)),
        kw_no_newline("%")
            .repeated()
            .at_least(1)
            .ignore_then(none_of("\r\n").repeated().collect::<String>())
            .map(MLtStatement::Comment),
        none_of(";\n")
            .repeated()
            .at_least(1)
            .collect::<String>()
            .then_ignore(just(';'))
            .padded_by(text::inline_whitespace())
            .map(MLtStatement::Error),
    )));

    let mlt_function_header = kw("function")
        .ignore_then(sident())
        .then_ignore(kw("="))
        .then(sident())
        .then(
            sident()
                .separated_by(kw(","))
                .collect()
                .delimited_by(kw("("), kw_no_newline(")")),
        );

    let mlt_function = mlt_function_header
        .then(mlt_statement.repeated().collect())
        .then_ignore(kw("end"))
        .map(|(((return_obj, name), params), body)| MLtFunction {
            return_obj,
            name,
            params,
            body,
        });

    return mlt_function;
}
