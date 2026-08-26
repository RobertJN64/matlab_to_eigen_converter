use crate::syntax::*;

// TODO - reject assigning to integer or function

pub fn transform_matrix_multisegment(value: MLtValue) -> MLtValue {
    match value.clone() {
        MLtValue::Matrix(mlt_matrix_access) => {
            if let MLtMatrixAccess::MatrixMultiSegment(name, segments) = mlt_matrix_access {
                MLtValue::InlineMatrix(
                    segments
                        .iter()
                        .map(|mlt_range| {
                            MLtExpr::Basic(MLtValue::Matrix(MLtMatrixAccess::MatrixSegment(
                                name.clone(),
                                mlt_range.clone(),
                            )))
                        })
                        .collect(),
                )
            } else {
                value
            }
        }
        MLtValue::StructMatrix(prefix, mlt_matrix_access) => {
            if let MLtMatrixAccess::MatrixMultiSegment(name, segments) = mlt_matrix_access {
                MLtValue::InlineMatrix(
                    segments
                        .iter()
                        .map(|mlt_range| {
                            MLtExpr::Basic(MLtValue::StructMatrix(
                                prefix.clone(),
                                MLtMatrixAccess::MatrixSegment(name.clone(), mlt_range.clone()),
                            ))
                        })
                        .collect(),
                )
            } else {
                value
            }
        }
        _ => value,
    }
}

// TODO - name this better to indicate the internal transforms
fn transform_pi(value: MLtValue) -> MLtValue {
    match value {
        MLtValue::Matrix(MLtMatrixAccess::Matrix(name)) => {
            if name == "pi" {
                MLtValue::Matrix(MLtMatrixAccess::Matrix("M_PI".to_string()))
            } else {
                MLtValue::Matrix(MLtMatrixAccess::Matrix(name))
            }
        }
        MLtValue::InlineMatrix(mlt_exprs) => {
            MLtValue::InlineMatrix(mlt_exprs.into_iter().map(transform_expression).collect())
        }
        MLtValue::FunctionCall(name, mlt_exprs) => MLtValue::FunctionCall(
            name,
            mlt_exprs.into_iter().map(transform_expression).collect(),
        ),
        _ => value,
    }
}

fn transform_matrix_index(value: MLtValue) -> MLtValue {
    let allowed_function_calls = vec!["ones", "zeros", "eye"];
    match value.clone() {
        MLtValue::FunctionCall(fname, mlt_exprs) => match mlt_exprs.as_slice() {
            [MLtExpr::Basic(MLtValue::Integer(idx))] => {
                if allowed_function_calls.contains(&fname.as_str()) {
                    value
                } else {
                    MLtValue::Matrix(MLtMatrixAccess::MatrixIndex(fname, *idx))
                }
            }
            _ => value,
        },
        _ => value,
    }
}

fn transform_value(value: MLtValue) -> MLtValue {
    transform_matrix_index(transform_pi(transform_matrix_multisegment(value)))
}

pub fn transform_expression(expr: MLtExpr) -> MLtExpr {
    match expr {
        MLtExpr::Basic(mlt_value) => MLtExpr::Basic(transform_value(mlt_value)),
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
        MLtValue::Matrix(MLtMatrixAccess::Matrix(target)),
        MLtExpr::BinOp(dividend_expr, MLtBinOp::Div, r_expr),
    ) = &statement
    {
        if let MLtExpr::Basic(MLtValue::Matrix(MLtMatrixAccess::Matrix(ref dividend))) =
            **dividend_expr
        {
            if let MLtExpr::Basic(MLtValue::FunctionCall(ref fname, ref args)) = **r_expr {
                if fname == "norm"
                    && args.len() == 1
                    && matches!(&args[0], MLtExpr::Basic(MLtValue::Matrix(MLtMatrixAccess::Matrix(arg))) if arg == dividend && arg == target)
                {
                    return MLtStatement::Normalization(target.clone());
                }
            }
        }
    }

    if let MLtStatement::IfStatement(expr, body) = statement {
        return MLtStatement::IfStatement(
            transform_expression(expr),
            body.into_iter()
                .map(|s| transform_statement(s, persistent_params))
                .collect(),
        );
    }

    if let MLtStatement::Assignment(left, right) = statement {
        return MLtStatement::Assignment(transform_value(left), transform_expression(right));
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
