use std::collections::HashMap;

use crate::syntax::*;

// returns the type (rows, cols) of a matlab expression so the C++ type can be inserted

pub fn inline_matrix_type(
    exprs: &Vec<MLtExpr>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> (u32, u32) {
    let (mut rows, cols) = expr_type(
        exprs
            .get(0)
            .expect("Inline matrix must have at least one element"),
        ti_state,
        line_num,
    );
    for expr in exprs.iter().skip(1) {
        let (new_rows, new_cols) = expr_type(expr, ti_state, line_num);
        if cols != new_cols {
            println!(
                "Inline matrix type warning: concat: {} by {} with {} by {} on line {}.",
                rows, cols, new_rows, new_cols, line_num
            );
        }
        rows += new_rows;
    }
    (rows, cols)
}

fn matrix_type(
    prefix: &str,
    matrix: &MLtMatrixAccess,
    ti_state: &mut HashMap<String, (u32, u32)>,
) -> (u32, u32) {
    match matrix {
        MLtMatrixAccess::Matrix(name) => {
            if let Some((rows, cols)) = ti_state.get(format!("{}{}", prefix, name).as_str()) {
                (*rows, *cols)
            } else {
                println!("Couldn't find {}{} in types", prefix, name);
                (0, 0)
            }
        }
        MLtMatrixAccess::MatrixIndex(_, _) => (1, 1),
        MLtMatrixAccess::MatrixSegment(_, mlt_range) => (mlt_range.end - mlt_range.start + 1, 1),
        MLtMatrixAccess::MatrixMultiSegment(_, _) => {
            panic!("MatrixMultiSegment should be converted to an inline matrix")
        }
        MLtMatrixAccess::MatrixBlock(_, rows, cols) => {
            (rows.end - rows.start + 1, cols.end - cols.start + 1)
        }
    }
}

pub fn lvalue_type(
    lvalue: &MLtLValue,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> (u32, u32) {
    match lvalue {
        MLtLValue::Integer(_) | MLtLValue::Float(_) => (1, 1),
        MLtLValue::Matrix(matrix) => matrix_type("", matrix, ti_state),
        MLtLValue::StructMatrix(prefix, matrix) => {
            matrix_type(format!("{}.", prefix).as_str(), matrix, ti_state)
        }
        MLtLValue::InlineMatrix(lvalues) => inline_matrix_type(lvalues, ti_state, line_num),
        MLtLValue::FunctionCall(function_name, function_params) => match function_name.as_str() {
            "eye" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(n))) = function_params.get(0) {
                    let n = n.parse().expect("Argument to eye must be an int");
                    return (n, n);
                }
                panic!("eye expects one integer argument");
            }
            "ones" | "zeros" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(rows))) = function_params.get(0) {
                    if let Some(MLtExpr::Basic(MLtLValue::Integer(cols))) = function_params.get(1) {
                        let rows = rows.parse().expect("Argument to ones|zeros must be an int");
                        let cols = cols.parse().expect("Argument to ones|zeros must be an int");
                        return (rows, cols);
                    } else {
                        let rows_cols =
                            rows.parse().expect("Argument to ones|zeros must be an int");
                        return (rows_cols, rows_cols);
                    }
                }
                panic!("ones|zeros expects two integer arguments");
            }
            // same size as the left arg
            "expm" | "min" | "max" | "cross" | "abs" | "exp" => {
                if let Some(expr) = function_params.get(0) {
                    let (rows, cols) = expr_type(expr, ti_state, line_num);
                    return (rows, cols);
                }
                panic!("expm|min|max|cross|abs|exp expects at least one matrix argument");
            }
            "norm" => (1, 1),
            "diag" => {
                if let Some(expr) = function_params.get(0) {
                    let (rows, cols) = expr_type(expr, ti_state, line_num);
                    if cols == 1 {
                        return (rows, rows);
                    }
                }
                panic!("diag expects one vector argument");
            }
            fname => {
                if let Some((rows, cols)) = ti_state.get(fname) {
                    (*rows, *cols)
                } else {
                    println!("Couldn't find {} in functions", fname);
                    (0, 0)
                }
            }
        },
    }
}

pub fn expr_type(
    expr: &MLtExpr,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> (u32, u32) {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_type(mlt_lvalue, ti_state, line_num),
        MLtExpr::Negation(mlt_expr) => expr_type(mlt_expr, ti_state, line_num),
        MLtExpr::Transposed(mlt_expr) => {
            let (cols, rows) = expr_type(mlt_expr, ti_state, line_num);
            (rows, cols) // transpose reverses the order
        }
        MLtExpr::Parenthesized(mlt_expr) => expr_type(mlt_expr, ti_state, line_num),
        MLtExpr::BinOp(left, mlt_bin_op, right) => {
            match mlt_bin_op {
                MLtBinOp::Add | MLtBinOp::Sub => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num);
                    let (rrows, rcols) = expr_type(right, ti_state, line_num);
                    if lrows != rrows || lcols != rcols {
                        println!(
                            "Matrix add/sub type warning: {} by {} +/- {} by {} on line {}.",
                            lrows, lcols, rrows, rcols, line_num
                        );
                    }
                    (lrows, lcols)
                }
                MLtBinOp::Mul => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num);
                    let (rrows, rcols) = expr_type(right, ti_state, line_num);
                    if lrows == 1 && lcols == 1 {
                        // mul by scalar
                        (rrows, rcols)
                    } else if rrows == 1 && rcols == 1 {
                        // mul by scalar
                        (lrows, lcols)
                    } else {
                        if lcols != rrows {
                            println!(
                                "Matrix mul type warning: {} by {} * {} by {} on line {}.",
                                lrows, lcols, rrows, rcols, line_num
                            );
                        }
                        (lrows, rcols)
                    }
                }
                MLtBinOp::Div => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num);
                    let (rrows, rcols) = expr_type(right, ti_state, line_num);
                    if rrows == 1 && rcols == 1 {
                        // division by scalar
                        (lrows, lcols)
                    } else {
                        // same as multiplying by the inverse, which doesn't change the size
                        if lcols != rrows {
                            println!(
                                "Matrix div type warning: {} by {} / {} by {} on line {}.",
                                lrows, lcols, rrows, rcols, line_num
                            );
                        }
                        (lrows, rcols)
                    }
                }
                MLtBinOp::Pow | MLtBinOp::CwisePow => expr_type(left, ti_state, line_num),
                MLtBinOp::CwiseMul | MLtBinOp::CwiseDiv => expr_type(left, ti_state, line_num),
                MLtBinOp::And | MLtBinOp::Or => (1, 1), // float is basically a bool - TODO - check that inputs are bools
                MLtBinOp::EqualTo | MLtBinOp::NotEqualTo => (1, 1), // float is basically a bool - TODO - check that input shapes match
                MLtBinOp::LessThan
                | MLtBinOp::LessThanEqualTo
                | MLtBinOp::GreaterThan
                | MLtBinOp::GreaterThanEqualTo => (1, 1),
            }
        }
    }
}
