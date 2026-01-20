use crate::syntax::*;

// returns the type (rows, cols) of a matlab expression so the C++ type can be inserted

fn matrix_type(matrix: &MLtMatrixAccess) -> (u32, u32) {
    match matrix {
        MLtMatrixAccess::Matrix(_) => (0, 0), // TODO - look these up
        MLtMatrixAccess::MatrixSegment(_, mlt_range) => (mlt_range.end - mlt_range.start + 1, 1),
        MLtMatrixAccess::MatrixBlock(_, rows, cols) => {
            (rows.end - rows.start + 1, cols.end - cols.start + 1)
        }
    }
}

fn lvalue_type(lvalue: &MLtLValue) -> (u32, u32) {
    match lvalue {
        MLtLValue::Integer(_) => (1, 1),
        MLtLValue::Float(_, _) => (1, 1),
        MLtLValue::Matrix(matrix) => matrix_type(matrix),
        MLtLValue::StructMatrix(_, matrix) => matrix_type(matrix),
        MLtLValue::InlineMatrix(_) => (0, 0), // TODO - add these up
        MLtLValue::FunctionCall(function_name, function_params) => match function_name.as_str() {
            "eye" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(n))) = function_params.get(0) {
                    let n = n.parse().expect("Argument to eye must be an int");
                    return (n, n);
                }
                panic!("eye expects one integer argument");
            }
            "zeros" => {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(rows))) = function_params.get(0) {
                    if let Some(MLtExpr::Basic(MLtLValue::Integer(cols))) = function_params.get(1) {
                        let rows = rows.parse().expect("Argument to zeros must be an int");
                        let cols = cols.parse().expect("Argument to zeros must be an int");
                        return (rows, cols);
                    }
                }
                panic!("zeros expects two integer arguments");
            }
            _ => (0, 0), // TODO - handle unknown function
        },
    }
}

pub fn expr_type(expr: &MLtExpr) -> (u32, u32) {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_type(mlt_lvalue),
        MLtExpr::Transposed(mlt_expr) => {
            let (cols, rows) = expr_type(mlt_expr);
            (rows, cols) // transpose reverses the order
        }
        MLtExpr::Parenthesized(mlt_expr) => expr_type(mlt_expr),
        MLtExpr::BinOp(mlt_expr, mlt_bin_op, mlt_expr1) => (0, 0), // TODO - look these up
    }
}
