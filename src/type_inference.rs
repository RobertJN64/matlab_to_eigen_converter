use std::collections::HashMap;
use std::fmt::Write;

use crate::error::{TranspilerError, TypeParseError};
use crate::syntax::*;

// returns the type (rows, cols) of a matlab expression so the C++ type can be inserted

pub fn inline_matrix_type(
    exprs: &Vec<MLtExpr>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> Result<(u32, u32), TranspilerError> {
    let (mut rows, cols) = expr_type(
        // .expect() guaranteed by parsing logic
        exprs
            .get(0)
            .expect("Inline matrix must have at least one element"),
        ti_state,
        line_num,
        warnings,
    )?;
    for expr in exprs.iter().skip(1) {
        let (new_rows, new_cols) = expr_type(expr, ti_state, line_num, warnings)?;
        if cols != new_cols {
            let _ = writeln!(
                warnings,
                "Inline matrix type warning: concat: {} by {} with {} by {} on line {}.",
                rows, cols, new_rows, new_cols, line_num
            );
        }
        rows += new_rows;
    }
    Ok((rows, cols))
}

fn matrix_type(
    prefix: &str,
    matrix: &MLtMatrixAccess,
    ti_state: &mut HashMap<String, (u32, u32)>,
    warnings: &mut String,
) -> Result<(u32, u32), TranspilerError> {
    Ok(match matrix {
        MLtMatrixAccess::Matrix(name) => {
            if let Some((rows, cols)) = ti_state.get(format!("{}{}", prefix, name).as_str()) {
                (*rows, *cols)
            } else {
                let _ = writeln!(warnings, "Couldn't find {}{} in types", prefix, name);
                (0, 0)
            }
        }
        MLtMatrixAccess::MatrixIndex(name, idx) => {
            if let Some((rows, cols)) = ti_state.get(format!("{}{}", prefix, name).as_str()) {
                if *cols == 1 {
                    if *idx > *rows {
                        let _ = writeln!(
                            warnings,
                            "Matrix index type warning - tried to access index at {} on {}{} which has {} rows",
                            *idx, prefix, name, *rows
                        );
                    }
                } else {
                    let _ = writeln!(
                        warnings,
                        "Matrix index type warning - {}{} is not a vector",
                        prefix, name
                    );
                }
            } else {
                let _ = writeln!(
                    warnings,
                    "Matrix index type warning - couldn't find {}{} in types so can't perform matrix bounds check",
                    prefix, name
                );
            }
            (1, 1)
        }
        MLtMatrixAccess::MatrixSegment(name, mlt_range) => {
            if let Some((rows, cols)) = ti_state.get(format!("{}{}", prefix, name).as_str()) {
                if *cols == 1 {
                    if mlt_range.end > *rows {
                        let _ = writeln!(
                            warnings,
                            "Matrix segment type warning - tried to access segment ending at {} on {}{} which has {} rows",
                            mlt_range.end, prefix, name, *rows
                        );
                    }
                } else {
                    let _ = writeln!(
                        warnings,
                        "Matrix segment type warning - {}{} is not a vector",
                        prefix, name
                    );
                }
            } else {
                let _ = writeln!(
                    warnings,
                    "Matrix segment type warning - couldn't find {}{} in types so can't perform matrix bounds check",
                    prefix, name
                );
            }
            (mlt_range.end - mlt_range.start + 1, 1)
        }
        MLtMatrixAccess::MatrixMultiSegment(_, _) => {
            // panic() guaranteed by transform logic
            panic!("MatrixMultiSegment should be converted to an inline matrix")
        }
        MLtMatrixAccess::MatrixBlock(name, row_range, col_range) => {
            if let Some((rows, cols)) = ti_state.get(format!("{}{}", prefix, name).as_str()) {
                if row_range.end > *rows || col_range.end > *cols {
                    let _ = writeln!(
                        warnings,
                        "Matrix block type warning - tried to access block ending at {},{} on {}{} which has size {},{}",
                        row_range.end, col_range.end, prefix, name, *rows, *cols
                    );
                }
            } else {
                let _ = writeln!(
                    warnings,
                    "Matrix block type warning - couldn't find {}{} in types so can't perform matrix bounds check",
                    prefix, name
                );
            }
            (
                row_range.end - row_range.start + 1,
                col_range.end - col_range.start + 1,
            )
        }
    })
}

pub fn lvalue_type(
    lvalue: &MLtLValue,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> Result<(u32, u32), TranspilerError> {
    Ok(match lvalue {
        MLtLValue::Integer(_) | MLtLValue::Float(_) => (1, 1),
        MLtLValue::Matrix(matrix) => matrix_type("", matrix, ti_state, warnings)?,
        MLtLValue::StructMatrix(prefix, matrix) => {
            matrix_type(format!("{}.", prefix).as_str(), matrix, ti_state, warnings)?
        }
        MLtLValue::InlineMatrix(lvalues) => {
            inline_matrix_type(lvalues, ti_state, line_num, warnings)?
        }
        MLtLValue::FunctionCall(function_name, function_params) => match function_name.as_str() {
            "eye" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(n))) = function_params.get(0) {
                    let n = n.parse().map_err(|_| {
                        TranspilerError("Error: argument to eye must be an int.".to_string())
                    })?;
                    (n, n)
                } else {
                    Err(TranspilerError(
                        "Type Deduction Error: eye expects one integer argument.".to_string(),
                    ))?
                }
            }
            "ones" | "zeros" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(rows))) = function_params.get(0) {
                    if let Some(MLtExpr::Basic(MLtLValue::Integer(cols))) = function_params.get(1) {
                        let rows = rows.parse().map_err(|_| {
                            TranspilerError(
                                "Error: argument to ones|zeros must be an int.".to_string(),
                            )
                        })?;
                        let cols = cols.parse().map_err(|_| {
                            TranspilerError(
                                "Error: argument to ones|zeros must be an int.".to_string(),
                            )
                        })?;
                        (rows, cols)
                    } else {
                        let rows_cols = rows.parse().map_err(|_| {
                            TranspilerError(
                                "Error: argument to ones|zeros must be an int.".to_string(),
                            )
                        })?;
                        (rows_cols, rows_cols)
                    }
                } else {
                    Err(TranspilerError(
                        "Type Deduction Error: ones|zeros expects one or two integer arguments."
                            .to_string(),
                    ))?
                }
            }
            // same size as the left arg
            "expm" | "min" | "max" | "cross" | "abs" | "exp" => {
                if let Some(expr) = function_params.get(0) {
                    expr_type(expr, ti_state, line_num, warnings)?
                } else {
                    Err(TranspilerError(
                        "Type Deduction Error: expm|min|max|cross|abs|exp expects at least one matrix argument."
                            .to_string(),
                    ))?
                }
            }
            "norm" => (1, 1),
            "diag" => {
                if let Some(expr) = function_params.get(0) {
                    let (rows, cols) = expr_type(expr, ti_state, line_num, warnings)?;
                    if cols == 1 {
                        return Ok((rows, rows));
                    }
                }
                return Err(TranspilerError(
                    "Type Deduction Error: diag expects one vector argument.".to_string(),
                ));
            }
            fname => {
                if let Some((rows, cols)) = ti_state.get(fname) {
                    (*rows, *cols)
                } else {
                    let _ = writeln!(warnings, "Couldn't find {} in functions", fname);
                    (0, 0)
                }
            }
        },
    })
}

pub fn expr_type(
    expr: &MLtExpr,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> Result<(u32, u32), TranspilerError> {
    Ok(match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_type(mlt_lvalue, ti_state, line_num, warnings)?,
        MLtExpr::Negation(mlt_expr) => expr_type(mlt_expr, ti_state, line_num, warnings)?,
        MLtExpr::Transposed(mlt_expr) => {
            let (cols, rows) = expr_type(mlt_expr, ti_state, line_num, warnings)?;
            (rows, cols) // transpose reverses the order
        }
        MLtExpr::Parenthesized(mlt_expr) => expr_type(mlt_expr, ti_state, line_num, warnings)?,
        MLtExpr::BinOp(left, mlt_bin_op, right) => {
            match mlt_bin_op {
                MLtBinOp::Add | MLtBinOp::Sub => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num, warnings)?;
                    let (rrows, rcols) = expr_type(right, ti_state, line_num, warnings)?;
                    if lrows != rrows || lcols != rcols {
                        let _ = writeln!(
                            warnings,
                            "Matrix add/sub type warning: {} by {} +/- {} by {} on line {}.",
                            lrows, lcols, rrows, rcols, line_num
                        );
                    }
                    (lrows, lcols)
                }
                MLtBinOp::Mul => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num, warnings)?;
                    let (rrows, rcols) = expr_type(right, ti_state, line_num, warnings)?;
                    if lrows == 1 && lcols == 1 {
                        // mul by scalar
                        (rrows, rcols)
                    } else if rrows == 1 && rcols == 1 {
                        // mul by scalar
                        (lrows, lcols)
                    } else {
                        if lcols != rrows {
                            let _ = writeln!(
                                warnings,
                                "Matrix mul type warning: {} by {} * {} by {} on line {}.",
                                lrows, lcols, rrows, rcols, line_num
                            );
                        }
                        (lrows, rcols)
                    }
                }
                MLtBinOp::Div => {
                    let (lrows, lcols) = expr_type(left, ti_state, line_num, warnings)?;
                    let (rrows, rcols) = expr_type(right, ti_state, line_num, warnings)?;
                    if rrows == 1 && rcols == 1 {
                        // division by scalar
                        (lrows, lcols)
                    } else {
                        // same as multiplying by the inverse, which doesn't change the size
                        if lcols != rrows {
                            let _ = writeln!(
                                warnings,
                                "Matrix div type warning: {} by {} / {} by {} on line {}.",
                                lrows, lcols, rrows, rcols, line_num
                            );
                        }
                        (lrows, rcols)
                    }
                }
                MLtBinOp::Pow | MLtBinOp::CwisePow => {
                    expr_type(left, ti_state, line_num, warnings)?
                }
                MLtBinOp::CwiseMul | MLtBinOp::CwiseDiv => {
                    expr_type(left, ti_state, line_num, warnings)?
                }
                MLtBinOp::And | MLtBinOp::Or => (1, 1), // float is basically a bool - TODO - check that inputs are bools
                MLtBinOp::EqualTo | MLtBinOp::NotEqualTo => (1, 1), // float is basically a bool - TODO - check that input shapes match
                MLtBinOp::LessThan
                | MLtBinOp::LessThanEqualTo
                | MLtBinOp::GreaterThan
                | MLtBinOp::GreaterThanEqualTo => (1, 1),
            }
        }
    })
}

pub fn name_to_type(name: &str) -> Result<(u32, u32), TypeParseError> {
    match name {
        "float" | "double" | "int" | "bool" => Ok((1, 1)),

        s if let Some(n) = s.strip_prefix("Vector") => {
            let n = n.parse().map_err(|_| {
                TypeParseError(format!("Vector types should be written like Vector#."))
            })?;
            Ok((n, 1))
        }

        s if let Some(dims) = s.strip_prefix("Matrix") => {
            let (rows, cols) = dims.split_once('_').ok_or_else(|| {
                TypeParseError(format!("Matrix types should be written like Matrix#_#."))
            })?;

            let rows = rows.parse().map_err(|_| {
                TypeParseError(format!("Matrix types should be written like Matrix#_#."))
            })?;

            let cols = cols.parse().map_err(|_| {
                TypeParseError(format!("Matrix types should be written like Matrix#_#."))
            })?;

            Ok((rows, cols))
        }

        _ => Err(TypeParseError(format!(
            "Type should be Matrix, Vector, float, double, int or bool."
        ))),
    }
}

pub fn parse_type(line: &str) -> Result<(&str, (u32, u32)), TypeParseError> {
    let (first, second) = line.split_once(": ").ok_or(TypeParseError(
        "Types should be written as <name: type>.".to_string(),
    ))?;

    return Ok((first, name_to_type(second)?));
}
