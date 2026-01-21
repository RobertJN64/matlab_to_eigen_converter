use crate::syntax::*;
use crate::type_inference::{expr_type, inline_matrix_type, lvalue_type};
use std::collections::HashMap;
use std::{fs::File, io::Write};

fn type_to_cpp((rows, cols): (u32, u32)) -> String {
    match (rows, cols) {
        (1, 1) => "float".to_string(),
        (rows, 1) => format!("Vector{}", rows),
        (rows, cols) => format!("Matrix{}_{}", rows, cols),
    }
}

fn matrix_to_cpp(matrix: MLtMatrixAccess) -> String {
    match matrix {
        MLtMatrixAccess::Matrix(ident) => ident,
        MLtMatrixAccess::MatrixSegment(ident, mlt_range) => {
            let range_width = mlt_range.end - mlt_range.start + 1;
            format!(
                "{}.segment<{}>({})",
                ident,
                range_width,
                mlt_range.start - 1
            )
        }
        MLtMatrixAccess::MatrixMultiSegment(_, _) => {
            panic!("MatrixMultiSegment should be converted to an inline matrix")
        }
        MLtMatrixAccess::MatrixBlock(ident, mlt_range_l, mlt_range_r) => {
            let range_width_l = mlt_range_l.end - mlt_range_l.start + 1;
            let range_width_r = mlt_range_r.end - mlt_range_r.start + 1;
            format!(
                "{}.block<{}, {}>({}, {})",
                ident,
                range_width_l,
                range_width_r,
                mlt_range_l.start - 1,
                mlt_range_r.start - 1
            )
        }
    }
}

fn function_call_to_cpp(
    function_name: String,
    function_params: Vec<MLtExpr>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    match function_name.as_str() {
        "eye" => {
            if let Some(MLtExpr::Basic(MLtLValue::Integer(n))) = function_params.get(0) {
                let n = n.parse().expect("Argument to eye must be an int");
                format!("{}::Identity()", type_to_cpp((n, n)))
            } else {
                panic!("eye expects one integer argument");
            }
        }
        "zeros" => {
            if let Some(MLtExpr::Basic(MLtLValue::Integer(rows))) = function_params.get(0) {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(cols))) = function_params.get(1) {
                    let rows = rows.parse().expect("Argument to zeros must be an int");
                    let cols = cols.parse().expect("Argument to zeros must be an int");
                    return format!("{}::Zero()", type_to_cpp((rows, cols)));
                }
            }
            panic!("zeros expects two integer arguments");
        }
        "ones" => {
            if let Some(MLtExpr::Basic(MLtLValue::Integer(rows))) = function_params.get(0) {
                if let Some(MLtExpr::Basic(MLtLValue::Integer(cols))) = function_params.get(1) {
                    let rows = rows.parse().expect("Argument to ones must be an int");
                    let cols = cols.parse().expect("Argument to ones must be an int");
                    return format!("{}::Ones()", type_to_cpp((rows, cols)));
                }
            }
            panic!("ones expects two integer arguments");
        }
        "expm" => format!(
            "matrixExpPade6({})",
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p, ti_state, line_num))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "diag" => format!(
            "({}).asDiagonal()",
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p, ti_state, line_num))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "{}({})",
            function_name,
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p, ti_state, line_num))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn lvalue_to_cpp(
    lvalue: MLtLValue,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    match lvalue {
        MLtLValue::Integer(val) | MLtLValue::Float(val) => format!("{}", val),
        MLtLValue::Matrix(matrix) => matrix_to_cpp(matrix),
        MLtLValue::StructMatrix(struct_name, matrix) => {
            format!("{}.{}", struct_name, matrix_to_cpp(matrix))
        }
        MLtLValue::InlineMatrix(mlt_exprs) => format!(
            "({}() << {}).finished()",
            type_to_cpp(inline_matrix_type(&mlt_exprs, ti_state, line_num)),
            mlt_exprs
                .into_iter()
                .map(|v| expr_to_cpp(v, ti_state, line_num))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        MLtLValue::FunctionCall(function_name, function_params) => {
            function_call_to_cpp(function_name, function_params, ti_state, line_num)
        }
    }
}

fn binop_to_cpp(binop: MLtBinOp) -> &'static str {
    match binop {
        MLtBinOp::Add => "+",
        MLtBinOp::Sub => "-",
        MLtBinOp::Mul => "*",
        MLtBinOp::Div => "/",
        MLtBinOp::Pow => "^",
        MLtBinOp::And => "&&",
        MLtBinOp::Or => "||",
        MLtBinOp::EqualTo => "==",
        MLtBinOp::NotEqualTo => "!=",
    }
}

fn expr_to_cpp(
    expr: MLtExpr,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_to_cpp(mlt_lvalue, ti_state, line_num),
        MLtExpr::Transposed(mlt_expr) => {
            format!("{}.transpose()", expr_to_cpp(*mlt_expr, ti_state, line_num))
        }
        MLtExpr::Parenthesized(mlt_expr) => {
            format!("({})", expr_to_cpp(*mlt_expr, ti_state, line_num))
        }
        MLtExpr::BinOp(mlt_exprl, mlt_bin_op, mlt_exprr) => {
            // if dividing by a matrix mul by the inverse instead
            if matches!(mlt_bin_op, MLtBinOp::Div)
                && !matches!(
                    *mlt_exprr,
                    MLtExpr::Basic(MLtLValue::Integer(_) | MLtLValue::Float(_))
                )
            {
                format!(
                    "{} * {}.inverse()",
                    expr_to_cpp(*mlt_exprl, ti_state, line_num),
                    expr_to_cpp(*mlt_exprr, ti_state, line_num)
                )
            } else if matches!(mlt_bin_op, MLtBinOp::Pow) {
                format!(
                    "pow({}, {})",
                    expr_to_cpp(*mlt_exprl, ti_state, line_num),
                    expr_to_cpp(*mlt_exprr, ti_state, line_num)
                )
            } else {
                format!(
                    "{} {} {}",
                    expr_to_cpp(*mlt_exprl, ti_state, line_num),
                    binop_to_cpp(mlt_bin_op),
                    expr_to_cpp(*mlt_exprr, ti_state, line_num)
                )
            }
        }
    }
}

fn matrix_access_should_have_type(matrix: &MLtMatrixAccess) -> bool {
    match matrix {
        MLtMatrixAccess::Matrix(_) => true,
        MLtMatrixAccess::MatrixSegment(_, _) => false,
        MLtMatrixAccess::MatrixMultiSegment(_, _) => false,
        MLtMatrixAccess::MatrixBlock(_, _, _) => false,
    }
}

fn lvalue_is_simple_matrix(lvalve: &MLtLValue) -> bool {
    match lvalve {
        MLtLValue::Integer(_) | MLtLValue::Float(_) => false,
        MLtLValue::Matrix(mlt_matrix_access) => matrix_access_should_have_type(mlt_matrix_access),
        MLtLValue::StructMatrix(_, mlt_matrix_access) => {
            matrix_access_should_have_type(mlt_matrix_access)
        }
        MLtLValue::InlineMatrix(_) => false,
        MLtLValue::FunctionCall(_, _) => false,
    }
}

fn generate_output_for_statement(
    statement: MLtStatement,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    match statement {
        MLtStatement::Assignment(lvalue, expr) => {
            let simple_matrix = lvalue_is_simple_matrix(&lvalue); // we don't place types on matrix accesses
            let left_side_cpp = lvalue_to_cpp(lvalue.clone(), ti_state, line_num);
            let right_side_type = expr_type(&expr, ti_state, line_num);
            let right_side_cpp = expr_to_cpp(expr, ti_state, line_num);

            // don't apply type if we already have a type recorded
            if simple_matrix && !ti_state.contains_key(&left_side_cpp) {
                ti_state.insert(left_side_cpp.clone(), right_side_type);
                format!(
                    "{} {} = {};",
                    type_to_cpp(right_side_type),
                    left_side_cpp,
                    right_side_cpp
                )
            } else {
                let left_side_type = lvalue_type(&lvalue, ti_state, line_num);
                if left_side_type != right_side_type {
                    println!(
                        "Assignment type warning: left side type does not match right side type: ({}, {}) != ({}, {}) on line {}.",
                        left_side_type.0,
                        left_side_type.1,
                        right_side_type.0,
                        right_side_type.1,
                        line_num
                    )
                }
                format!("{} = {};", left_side_cpp, right_side_cpp)
            }
        }
        MLtStatement::Normalization(matrix_name) => {
            format!("{}.normalize();", matrix_name)
        }
        MLtStatement::Persistent(idents) => {
            *line_num += 1;
            format!(
                "// the following vars are persistent: {}\n",
                idents.join(", ")
            )
        }
        MLtStatement::IfStatement(mlt_expr, mlt_statements) => {
            *line_num += 1;
            let text = format!(
                "if ({}) {{\n{}}}",
                expr_to_cpp(mlt_expr, ti_state, line_num),
                // clone ti_state here to prevent types from propagating outside the if statement
                generate_output_for_statement_list(mlt_statements, &mut ti_state.clone(), line_num)
            );
            text
        }
        MLtStatement::Comment(comment_str) => {
            format!("// {}", comment_str)
        }
        MLtStatement::Error(error_str) => {
            println!("Error line: {}", error_str);
            format!("// {}; // line could not be parsed", error_str)
        }
        MLtStatement::NewLine => {
            *line_num += 1;
            format!("\n")
        }
    }
}

fn generate_output_for_statement_list(
    statement_list: Vec<MLtStatement>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    statement_list
        .into_iter()
        .map(|s| generate_output_for_statement(s, ti_state, line_num))
        .collect()
}

fn generate_output_for_function(
    function: MLtFunction,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
) -> String {
    format!(
        "{} {}({}) {{{}return {};\n}}\n",
        type_to_cpp(
            *ti_state
                .get("_self")
                .expect("ti_state should have `_self` to represent function return type")
        ),
        function.name,
        function
            .params
            .into_iter()
            .map(|p| {
                let type_str = match ti_state.get(p.strip_prefix("&").unwrap_or(&p)) {
                    Some(t) => type_to_cpp(*t),
                    None => format!("{}_t", p.strip_prefix("&").unwrap_or(&p)),
                };
                format!("{} {}", type_str, p)
            })
            .collect::<Vec<String>>()
            .join(", "),
        generate_output_for_statement_list(function.body, ti_state, line_num),
        function.return_obj
    )
}

pub fn generate_output_file(function: MLtFunction, ti_state: &mut HashMap<String, (u32, u32)>) {
    let mut file = File::create("out.cpp").unwrap();
    let mut line_num = 3;
    let _ = file.write_all("#include \"matlab_funcs.h\"\n\n".as_bytes());
    let _ =
        file.write_all(generate_output_for_function(function, ti_state, &mut line_num).as_bytes());
}
