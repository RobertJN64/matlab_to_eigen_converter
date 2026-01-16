use crate::syntax::*;
use std::{fs::File, io::Write};

fn matrix_to_cpp(matrix: MLtMatrixAccess) -> String {
    match matrix {
        MLtMatrixAccess::Matrix(ident) => ident,
        MLtMatrixAccess::MatrixSegment(ident, mlt_range) => {
            format!("{}.segment<>({}:{})", ident, mlt_range.start, mlt_range.end) // TODO - update these for C++
        }
        MLtMatrixAccess::MatrixBlock(ident, mlt_range_l, mlt_range_r) => {
            format!(
                "{}({}:{}, {}:{})",
                ident, mlt_range_l.start, mlt_range_l.end, mlt_range_r.start, mlt_range_r.end
            )
        }
    }
}

fn function_call_to_cpp(function_name: String, function_params: Vec<MLtExpr>) -> String {
    match function_name.as_str() {
        "eye" => {
            if let Some(MLtExpr::Basic(MLtLValue::Integer(n))) = function_params.get(0) {
                format!("Matrix{}_{}::Identity()", n, n)
            } else {
                panic!("eye expects one integer argument");
            }
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
        MLtBinOp::NotEqualTo => "!=",
    }
}

fn expr_to_cpp(expr: MLtExpr) -> String {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_to_cpp(mlt_lvalue),
        MLtExpr::Transposed(mlt_expr) => format!("{}.tranpose()", expr_to_cpp(*mlt_expr)),
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

fn generate_output_for_statement(statement: MLtStatement) -> String {
    match statement {
        MLtStatement::Assignment(lvalue, expr) => {
            format!("{} = {};\n", lvalue_to_cpp(lvalue), expr_to_cpp(expr))
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
