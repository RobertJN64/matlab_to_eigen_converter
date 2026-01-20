use crate::syntax::*;
use crate::type_inference::expr_type;
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
            ) // TODO - update these for C++
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

fn function_call_to_cpp(function_name: String, function_params: Vec<MLtExpr>) -> String {
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
        _ => format!(
            "{}({})",
            function_name,
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn lvalue_to_cpp(lvalue: MLtLValue) -> String {
    match lvalue {
        MLtLValue::Integer(val) => format!("{}", val),
        MLtLValue::Float(int_v, float_v) => format!("{}.{}", int_v, float_v),
        MLtLValue::Matrix(matrix) => matrix_to_cpp(matrix),
        MLtLValue::StructMatrix(struct_name, matrix) => {
            format!("{}.{}", struct_name, matrix_to_cpp(matrix))
        }
        MLtLValue::InlineMatrix(mlt_lvalues) => format!(
            "/* [ {} ] */",
            mlt_lvalues
                .into_iter()
                .map(|v| expr_to_cpp(v))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        MLtLValue::FunctionCall(function_name, function_params) => {
            function_call_to_cpp(function_name, function_params)
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

fn expr_to_cpp(expr: MLtExpr) -> String {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_to_cpp(mlt_lvalue),
        MLtExpr::Transposed(mlt_expr) => format!("{}.transpose()", expr_to_cpp(*mlt_expr)),
        MLtExpr::Parenthesized(mlt_expr) => format!("({})", expr_to_cpp(*mlt_expr)),
        MLtExpr::BinOp(mlt_exprl, mlt_bin_op, mlt_exprr) => {
            format!(
                "{} {} {}",
                expr_to_cpp(*mlt_exprl),
                binop_to_cpp(mlt_bin_op),
                expr_to_cpp(*mlt_exprr)
            )
        }
    }
}

fn matrix_access_should_have_type(matrix: &MLtMatrixAccess) -> bool {
    match matrix {
        MLtMatrixAccess::Matrix(_) => true,
        MLtMatrixAccess::MatrixSegment(_, _) => false,
        MLtMatrixAccess::MatrixBlock(_, _, _) => false,
    }
}

fn lvalue_should_have_type(lvalve: &MLtLValue) -> bool {
    match lvalve {
        MLtLValue::Integer(_) => false,
        MLtLValue::Float(_, _) => false,
        MLtLValue::Matrix(mlt_matrix_access) => matrix_access_should_have_type(mlt_matrix_access),
        MLtLValue::StructMatrix(_, mlt_matrix_access) => {
            matrix_access_should_have_type(mlt_matrix_access)
        }
        MLtLValue::InlineMatrix(_) => false,
        MLtLValue::FunctionCall(_, _) => false,
    }
}

fn generate_output_for_statement(statement: MLtStatement) -> String {
    match statement {
        MLtStatement::Assignment(lvalue, expr) => {
            if lvalue_should_have_type(&lvalue) {
                format!(
                    "{} {} = {};\n",
                    type_to_cpp(expr_type(&expr)),
                    lvalue_to_cpp(lvalue),
                    expr_to_cpp(expr)
                )
            } else {
                format!("{} = {};\n", lvalue_to_cpp(lvalue), expr_to_cpp(expr))
            }
        }
        MLtStatement::Normalization(matrix_name) => {
            format!("{}.normalize();\n", matrix_name)
        }
        MLtStatement::Persistent(idents) => {
            format!(
                "// the following vars are persistent: {}\n",
                idents.join(", ")
            )
        }
        MLtStatement::IfStatement(mlt_expr, mlt_statements) => {
            format!(
                "if ({}) {{\n {} \n}}\n",
                expr_to_cpp(mlt_expr),
                generate_output_for_statement_list(mlt_statements)
            )
        }
        MLtStatement::Comment(comment_str) => format!("// {}\n", comment_str),
        MLtStatement::Error(error_str) => {
            println!("Error line: {}", error_str);
            format!("// {}; // line could not be parsed\n", error_str)
        }
    }
}

fn generate_output_for_statement_list(statement_list: Vec<MLtStatement>) -> String {
    statement_list
        .into_iter()
        .map(generate_output_for_statement)
        .collect()
}

// TODO - handle special functions
fn generate_output_for_function(function: MLtFunction) -> String {
    format!(
        "void {}({}) {{\n{}return {};\n}}",
        function.name,
        function.params.join(", "), // TODO - add in types
        generate_output_for_statement_list(function.body),
        function.return_obj
    )
}

pub fn generate_output_file(function: MLtFunction) {
    let mut file = File::create("out.cpp").unwrap();
    let _ = file.write_all(generate_output_for_function(function).as_bytes());
}
