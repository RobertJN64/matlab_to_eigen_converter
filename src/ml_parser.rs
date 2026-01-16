use crate::syntax::*;
use chumsky::{prelude::*, text::*};

// used for keywords
fn kw<'src>(s: &'static str) -> impl Parser<'src, &'src str, ()> + Clone {
    just(s).padded().ignored()
}

pub fn parser<'src>() -> impl Parser<'src, &'src str, MLtFunction> {
    let mlt_range = int(10)
        .then_ignore(kw(":"))
        .then(int(10))
        .map(|(start, end): (&str, &str)| MLtRange {
            start: start.parse().expect("failed to parse output of int to int"),
            end: end.parse().expect("failed to parse output of int to int"),
        });

    let mlt_matrix = choice((
        ident()
            .then(mlt_range.clone().delimited_by(kw("("), kw(")")))
            .map(|(ident, pf): (&str, MLtRange)| {
                MLtMatrixAccess::MatrixSegment(ident.to_string(), pf)
            }),
        ident()
            .then(
                mlt_range
                    .clone()
                    .then_ignore(kw(","))
                    .then(mlt_range)
                    .delimited_by(kw("("), kw(")")),
            )
            .map(|(ident, (range_1, range_2))| {
                MLtMatrixAccess::MatrixBlock(ident.to_string(), range_1, range_2)
            }),
        ident().map(|ident: &str| MLtMatrixAccess::Matrix(ident.to_string())),
    ));

    let mut mlt_lvalue = Recursive::declare();
    let mut mlt_expr = Recursive::declare();

    mlt_lvalue.define(choice((
        int(10)
            .map(String::from)
            .then_ignore(just("."))
            .then(int(10).map(String::from))
            .map(|(int_v, float_v)| MLtLValue::Float(int_v, float_v)),
        int(10).map(String::from).map(MLtLValue::Integer),
        ident()
            .then(
                mlt_expr
                    .clone()
                    .separated_by(kw(","))
                    .collect()
                    .delimited_by(kw("("), kw(")")),
            )
            .map(|(function_name, params)| {
                MLtLValue::FunctionCall(function_name.to_string(), params)
            }),
        ident()
            .then_ignore(kw("."))
            .then(mlt_matrix.clone())
            .map(|(struct_name, matrix)| MLtLValue::StructMatrix(struct_name.to_string(), matrix)),
        mlt_expr
            .clone()
            .separated_by(kw(";"))
            .collect()
            .delimited_by(kw("["), kw("]"))
            .map(|s| MLtLValue::InlineMatrix(s)),
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

        let transposed_atom = choice((
            atom.clone()
                .then_ignore(kw("'"))
                .map(|e| MLtExpr::Transposed(Box::new(e))),
            atom,
        ));

        let mul_div = transposed_atom.clone().foldl(
            choice((kw("*").to(MLtBinOp::Mul), kw("/").to(MLtBinOp::Div)))
                .then(transposed_atom)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        let add_sub = mul_div.clone().foldl(
            choice((kw("+").to(MLtBinOp::Add), kw("-").to(MLtBinOp::Sub)))
                .then(mul_div)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        );

        add_sub.clone().foldl(
            choice((kw("~=").to(MLtBinOp::NotEqualTo),))
                .then(add_sub)
                .repeated(),
            |l, (op, r)| MLtExpr::BinOp(Box::new(l), op, Box::new(r)),
        )
    });

    let mlt_assignment = mlt_lvalue
        .then_ignore(kw("="))
        .then(mlt_expr.clone())
        .then_ignore(kw(";"))
        .map(|(lvalue, expr)| MLtAssignment { lvalue, expr });

    let mut mlt_statement = Recursive::declare();

    mlt_statement.define(choice((
        mlt_assignment.map(MLtStatement::Assignment),
        kw("if")
            .ignore_then(mlt_expr)
            .padded()
            .then(mlt_statement.clone().repeated().collect())
            .then_ignore(kw("end"))
            .map(|(cond, body)| MLtStatement::IfStatement(cond, body)),
        kw("%")
            .repeated()
            .at_least(1)
            .ignore_then(none_of("\r\n").repeated().collect::<String>())
            .padded()
            .map(MLtStatement::Comment),
        none_of(";\n")
            .repeated()
            .at_least(1)
            .collect::<String>()
            .then_ignore(just(';'))
            .padded()
            .map(MLtStatement::Error),
    )));

    let mlt_function_header = kw("function")
        .ignore_then(ident())
        .then_ignore(kw("="))
        .then(ident())
        .then(
            ident()
                .map(String::from)
                .separated_by(kw(","))
                .collect()
                .delimited_by(kw("("), kw(")")),
        );
    let mlt_function = mlt_function_header
        .then(mlt_statement.repeated().collect())
        .then_ignore(kw("end"))
        .map(|(((return_obj, name), params), body)| MLtFunction {
            return_obj: return_obj.to_string(),
            name: name.to_string(),
            params,
            body,
        });

    return mlt_function;
}
