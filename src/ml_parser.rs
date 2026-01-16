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

    let mlt_lvalue = choice((
        int(10)
            .map(String::from)
            .then_ignore(just("."))
            .then(int(10).map(String::from))
            .map(|(int_v, float_v)| MLtLValue::Float((int_v, float_v))),
        int(10).map(String::from).map(MLtLValue::Integer),
        ident()
            .then(mlt_range.delimited_by(kw("("), kw(")")))
            .map(|(ident, pf): (&str, MLtRange)| MLtLValue::MatrixSegment(ident.to_string(), pf)),
        ident().map(|ident: &str| MLtLValue::Matrix(ident.to_string())),
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

    let mlt_statement = choice((
        mlt_assignment.map(MLtStatement::Assignment),
        kw("%")
            .repeated()
            .at_least(1)
            .ignore_then(none_of("\r\n").repeated().collect::<String>())
            .padded()
            .map(MLtStatement::Comment),
        none_of(";")
            .repeated()
            .at_least(1)
            .collect::<String>()
            .then_ignore(just(';'))
            .padded()
            .map(MLtStatement::Error),
    ));

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
