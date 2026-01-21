use crate::syntax::*;

pub fn transform_matrix_multisegment(lvalue: MLtLValue) -> MLtLValue {
    match lvalue.clone() {
        MLtLValue::Matrix(mlt_matrix_access) => {
            if let MLtMatrixAccess::MatrixMultiSegment(name, segments) = mlt_matrix_access {
                MLtLValue::InlineMatrix(
                    segments
                        .iter()
                        .map(|mlt_range| {
                            MLtExpr::Basic(MLtLValue::Matrix(MLtMatrixAccess::MatrixSegment(
                                name.clone(),
                                mlt_range.clone(),
                            )))
                        })
                        .collect(),
                )
            } else {
                lvalue
            }
        }
        MLtLValue::StructMatrix(prefix, mlt_matrix_access) => {
            if let MLtMatrixAccess::MatrixMultiSegment(name, segments) = mlt_matrix_access {
                MLtLValue::InlineMatrix(
                    segments
                        .iter()
                        .map(|mlt_range| {
                            MLtExpr::Basic(MLtLValue::StructMatrix(
                                prefix.clone(),
                                MLtMatrixAccess::MatrixSegment(name.clone(), mlt_range.clone()),
                            ))
                        })
                        .collect(),
                )
            } else {
                lvalue
            }
        }
        _ => lvalue,
    }
}

pub fn transform_expression(expr: MLtExpr) -> MLtExpr {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => MLtExpr::Basic(transform_matrix_multisegment(mlt_lvalue)),
        MLtExpr::Negation(mlt_expr) => MLtExpr::Negation(Box::new(transform_expression(*mlt_expr))),
        MLtExpr::Transposed(mlt_expr) => {
            MLtExpr::Transposed(Box::new(transform_expression(*mlt_expr)))
        }
        MLtExpr::Parenthesized(mlt_expr) => {
            MLtExpr::Parenthesized(Box::new(transform_expression(*mlt_expr)))
        }
        MLtExpr::BinOp(mlt_exprl, mlt_bin_op, mlt_exprr) => MLtExpr::BinOp(
            Box::new(transform_expression(*mlt_exprl)),
            mlt_bin_op,
            Box::new(transform_expression(*mlt_exprr)),
        ),
    }
}

fn transform_statement(
    statement: MLtStatement,
    persistent_params: &mut Vec<String>,
) -> MLtStatement {
    if let MLtStatement::Assignment(
        MLtLValue::Matrix(MLtMatrixAccess::Matrix(target)),
        MLtExpr::BinOp(dividend_expr, MLtBinOp::Div, r_expr),
    ) = &statement
    {
        if let MLtExpr::Basic(MLtLValue::Matrix(MLtMatrixAccess::Matrix(ref dividend))) =
            **dividend_expr
        {
            if let MLtExpr::Basic(MLtLValue::FunctionCall(ref fname, ref args)) = **r_expr {
                if fname == "norm"
                    && args.len() == 1
                    && matches!(&args[0], MLtExpr::Basic(MLtLValue::Matrix(MLtMatrixAccess::Matrix(arg))) if arg == dividend && arg == target)
                {
                    return MLtStatement::Normalization(target.clone());
                }
            }
        }
    }

    if let MLtStatement::IfStatement(expr, body) = statement {
        return MLtStatement::IfStatement(
            expr,
            body.into_iter()
                .map(|s| transform_statement(s, persistent_params))
                .collect(),
        );
    }

    if let MLtStatement::Assignment(left, right) = statement {
        return MLtStatement::Assignment(
            transform_matrix_multisegment(left),
            transform_expression(right),
        );
    }

    if let MLtStatement::Persistent(new_persis_params) = statement.clone() {
        persistent_params.extend(new_persis_params.into_iter().map(|s| format!("&{}", s)));
    }

    statement
}

pub fn transform_ast(mut function: MLtFunction) -> MLtFunction {
    let mut persistent_params = vec![];
    function.body = function
        .body
        .into_iter()
        .map(|s| transform_statement(s, &mut persistent_params))
        .collect();
    function.params.extend(persistent_params);

    function
}
