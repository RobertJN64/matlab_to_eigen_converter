use crate::syntax::*;
use std::{fmt::format, fs::File, io::Write};

fn lvalue_to_cpp(lvalue: MLtLValue) -> String {
    match lvalue {
        MLtLValue::Basic(ident) => ident,
        MLtLValue::Segment(ident, mlt_range) => {
            format!("{}({}:{})", ident, mlt_range.start, mlt_range.end) // TODO - update these for C++
        }
        MLtLValue::Block(ident, mlt_range_l, mlt_range_r) => {
            format!(
                "{}({}:{}, {}:{})",
                ident, mlt_range_l.start, mlt_range_l.end, mlt_range_r.start, mlt_range_r.end
            )
        }
    }
}

fn binop_to_cpp(binop: MLtBinOp) -> &'static str {
    match binop {
        MLtBinOp::Add => "+",
        MLtBinOp::Sub => "-",
        MLtBinOp::Mul => "*",
        MLtBinOp::Div => "/",
    }
}

fn expr_to_cpp(expr: MLtExpr) -> String {
    match expr {
        MLtExpr::Basic(mlt_lvalue) => lvalue_to_cpp(mlt_lvalue),
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

fn generate_output_for_assignment(assignment: MLtAssignment) -> String {
    format!(
        "{} = {};\n",
        lvalue_to_cpp(assignment.lvalue),
        expr_to_cpp(assignment.expr)
    )
}

fn generate_output_for_statement(statement: MLtStatement) -> String {
    match statement {
        MLtStatement::Assignment(mlt_assignment) => generate_output_for_assignment(mlt_assignment),
        MLtStatement::IfStatement(mlt_expr, mlt_statements) => {
            format!(
                "if ({}) {{\n {} \n}}\n",
                expr_to_cpp(mlt_expr),
                generate_output_for_statement_list(mlt_statements)
            )
        }
        MLtStatement::Comment(comment_str) => format!("// {}\n", comment_str),
        MLtStatement::Error(error_str) => {
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
