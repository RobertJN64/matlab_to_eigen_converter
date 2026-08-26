use crate::error::TranspilerError;
use crate::syntax::*;
use crate::type_inference::{expr_type, inline_matrix_type, value_type};
use std::collections::HashMap;
use std::fmt::Write;

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
        MLtMatrixAccess::MatrixIndex(ident, idx) => {
            format!("{}[{}]", ident, idx - 1)
        }
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
            // panic() guaranteed by transform logic
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

// diag(a) to a.asDiagonal()
fn function_to_dot_function(
    function_name: &str,
    function_params: Vec<MLtExpr>,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    let fname_map = HashMap::from([
        ("diag", "asDiagonal()"),
        ("abs", "cwiseAbs()"),
        ("norm", "norm()"),
        ("exp", "array().exp().matrix()"),
    ]);
    let dot_name = fname_map
        .get(function_name)
        .expect(format!("missing {} in dot_name map", function_name).as_str());
    Ok(match function_params.as_slice() {
        [MLtExpr::Basic(value)] => format!(
            "{}.{}",
            value_to_cpp(value.clone(), ti_state, line_num, warnings)?,
            dot_name
        ),
        [expr] => format!(
            "({}).{}",
            expr_to_cpp(expr.clone(), ti_state, line_num, warnings)?,
            dot_name
        ),
        _ => Err(TranspilerError(format!(
            "Error: {} must have exactly 1 arg",
            function_name
        )))?,
    })
}

// transform min(a, b) to a.cwiseMin(b)
fn function_to_chain_function(
    function_name: &str,
    function_params: Vec<MLtExpr>,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    let fname_map = HashMap::from([("min", "cwiseMin"), ("max", "cwiseMax"), ("cross", "cross")]);
    let chain_name = fname_map
        .get(function_name)
        .expect(format!("missing {} in chain_name map", function_name).as_str());

    if let Some(mlt_expr_l) = function_params.get(0) {
        if let Some(mlt_expr_r) = function_params.get(1) {
            if function_params.len() > 2 {
                let _ = writeln!(
                    warnings,
                    "Builtin function warning: ignoring extra arguments provided to {} on line {}.",
                    function_name, line_num
                );
            }

            return Ok(format!(
                "{}.{}({})",
                expr_to_cpp(mlt_expr_l.clone(), ti_state, line_num, warnings)?,
                chain_name,
                expr_to_cpp(mlt_expr_r.clone(), ti_state, line_num, warnings)?
            ));
        }
    }
    return Err(TranspilerError(format!(
        "Error: {} expects two arguments.",
        function_name
    )));
}

fn function_call_to_cpp(
    function_name: String,
    function_params: Vec<MLtExpr>,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    // when we call type inference again, use this to prevent duplicate warnings
    let mut not_warnings = String::new();

    Ok(match function_name.as_str() {
        "eye" => {
            format!(
                "{}::Identity()",
                type_to_cpp(value_type(
                    &MLtValue::FunctionCall(function_name, function_params),
                    ti_state,
                    line_num,
                    &mut not_warnings
                )?)
            )
        }
        "zeros" => {
            format!(
                "{}::Zero()",
                type_to_cpp(value_type(
                    &MLtValue::FunctionCall(function_name, function_params),
                    ti_state,
                    line_num,
                    &mut not_warnings
                )?)
            )
        }
        "ones" => {
            format!(
                "{}::Ones()",
                type_to_cpp(value_type(
                    &MLtValue::FunctionCall(function_name, function_params),
                    ti_state,
                    line_num,
                    &mut not_warnings
                )?)
            )
        }
        "expm" => format!(
            "matrixExpPade6({})",
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p, ti_state, line_num, warnings))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        "diag" | "abs" | "norm" | "exp" => function_to_dot_function(
            &function_name,
            function_params,
            ti_state,
            line_num,
            warnings,
        )?,
        "min" | "max" | "cross" => function_to_chain_function(
            &function_name,
            function_params,
            ti_state,
            line_num,
            warnings,
        )?,
        _ => format!(
            "{}({})",
            function_name,
            function_params
                .into_iter()
                .map(|p| expr_to_cpp(p, ti_state, line_num, warnings))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
    })
}

fn value_to_cpp(
    value: MLtValue,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    match value {
        MLtValue::Integer(val) => Ok(format!("{}", val)),
        MLtValue::Float(val) => Ok(format!("{}", val)),
        MLtValue::Matrix(matrix) => Ok(matrix_to_cpp(matrix)),
        MLtValue::StructMatrix(struct_name, matrix) => {
            Ok(format!("{}.{}", struct_name, matrix_to_cpp(matrix)))
        }
        MLtValue::InlineMatrix(mlt_exprs) => Ok(format!(
            "({}() << {}).finished()",
            type_to_cpp(inline_matrix_type(
                &mlt_exprs, ti_state, line_num, warnings
            )?),
            mlt_exprs
                .into_iter()
                .map(|v| expr_to_cpp(v, ti_state, line_num, warnings))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        MLtValue::FunctionCall(function_name, function_params) => {
            function_call_to_cpp(function_name, function_params, ti_state, line_num, warnings)
        }
    }
}

fn binop_to_cpp(binop: MLtBinOp) -> &'static str {
    match binop {
        MLtBinOp::Add => "+",
        MLtBinOp::Sub => "-",
        MLtBinOp::Mul => "*",
        MLtBinOp::CwiseMul => ".*",
        MLtBinOp::Div => "/",
        MLtBinOp::CwiseDiv => "./",
        MLtBinOp::Pow => "^",
        MLtBinOp::CwisePow => ".^",
        MLtBinOp::And => "&&",
        MLtBinOp::Or => "||",
        MLtBinOp::EqualTo => "==",
        MLtBinOp::NotEqualTo => "!=",
        MLtBinOp::LessThan => "<",
        MLtBinOp::LessThanEqualTo => "<=",
        MLtBinOp::GreaterThan => ">",
        MLtBinOp::GreaterThanEqualTo => ">=",
    }
}

fn expr_to_cpp(
    expr: MLtExpr,
    ti_state: &HashMap<String, (u32, u32)>,
    line_num: &u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    Ok(match expr {
        MLtExpr::Basic(mlt_value) => value_to_cpp(mlt_value, ti_state, line_num, warnings)?,
        MLtExpr::Negation(mlt_expr) => {
            format!("-{}", expr_to_cpp(*mlt_expr, ti_state, line_num, warnings)?)
        }
        MLtExpr::Transposed(mlt_expr) => format!(
            "{}.transpose()",
            expr_to_cpp(*mlt_expr, ti_state, line_num, warnings)?
        ),
        MLtExpr::Parenthesized(mlt_expr) => format!(
            "({})",
            expr_to_cpp(*mlt_expr, ti_state, line_num, warnings)?
        ),
        MLtExpr::BinOp(mlt_exprl, mlt_bin_op, mlt_exprr) => {
            // if dividing by a matrix mul by the inverse instead
            match mlt_bin_op {
                MLtBinOp::Div => {
                    if expr_type(&mlt_exprr, ti_state, line_num, warnings)? != (1, 1) {
                        format!(
                            "{} * {}.inverse()",
                            expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                            expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                        )
                    } else {
                        format!(
                            "{} {} {}",
                            expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                            binop_to_cpp(mlt_bin_op),
                            expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                        )
                    }
                }
                MLtBinOp::Pow => {
                    format!(
                        "pow({}, {})",
                        expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                        expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                    )
                }
                MLtBinOp::CwiseMul => {
                    format!(
                        "{}.cwiseProduct({})",
                        expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                        expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                    )
                }
                MLtBinOp::CwiseDiv => {
                    format!(
                        "{}.cwiseQuotient({})",
                        expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                        expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                    )
                }
                MLtBinOp::CwisePow => {
                    match *mlt_exprr {
                        MLtExpr::Basic(MLtValue::Integer(v)) => {
                            if v == 2 {
                                return Ok(format!(
                                    "{}.cwiseAbs2()",
                                    expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                                ));
                            }
                        }
                        _ => {}
                    }
                    Err(TranspilerError(
                        "Error: CwisePow only supports .^2".to_string(),
                    ))?
                }
                _ => {
                    format!(
                        "{} {} {}",
                        expr_to_cpp(*mlt_exprl, ti_state, line_num, warnings)?,
                        binop_to_cpp(mlt_bin_op),
                        expr_to_cpp(*mlt_exprr, ti_state, line_num, warnings)?
                    )
                }
            }
        }
    })
}

fn matrix_access_should_have_type(matrix: &MLtMatrixAccess) -> bool {
    match matrix {
        MLtMatrixAccess::Matrix(_) => true,
        MLtMatrixAccess::MatrixIndex(_, _) => false,
        MLtMatrixAccess::MatrixSegment(_, _) => false,
        MLtMatrixAccess::MatrixMultiSegment(_, _) => false,
        MLtMatrixAccess::MatrixBlock(_, _, _) => false,
    }
}

fn value_is_simple_matrix(lvalve: &MLtValue) -> bool {
    match lvalve {
        MLtValue::Integer(_) | MLtValue::Float(_) => false,
        MLtValue::Matrix(mlt_matrix_access) => matrix_access_should_have_type(mlt_matrix_access),
        MLtValue::StructMatrix(_, mlt_matrix_access) => {
            matrix_access_should_have_type(mlt_matrix_access)
        }
        MLtValue::InlineMatrix(_) => false,
        MLtValue::FunctionCall(_, _) => false,
    }
}

fn generate_output_for_statement(
    statement: MLtStatement,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> Result<String, TranspilerError> {
    Ok(match statement {
        MLtStatement::Assignment(value, expr) => {
            let simple_matrix = value_is_simple_matrix(&value); // we don't place types on matrix accesses
            let left_side_cpp = value_to_cpp(value.clone(), ti_state, line_num, warnings)?;
            let right_side_type = expr_type(&expr, ti_state, line_num, warnings)?;
            let right_side_cpp = expr_to_cpp(expr, ti_state, line_num, warnings)?;

            // The parser doesn't identify between assignable values (and it would be difficult because x(1)
            // could be an assignable matrix access or a function call depending on the type inference)
            // so instead emit a warning.
            match value {
                MLtValue::Integer(_) | MLtValue::Float(_) | MLtValue::FunctionCall(_, _) => {
                    let _ = writeln!(
                        warnings,
                        "Assignment warning: left side is not assignable: <{}> on line {}.",
                        left_side_cpp, line_num
                    );
                }
                MLtValue::Matrix(_) | MLtValue::StructMatrix(_, _) | MLtValue::InlineMatrix(_) => {}
            }

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
                let left_side_type = value_type(&value, ti_state, line_num, warnings)?;
                if left_side_type != right_side_type {
                    let _ = writeln!(
                        warnings,
                        "Assignment type warning: left side type does not match right side type: ({}, {}) != ({}, {}) on line {}.",
                        left_side_type.0,
                        left_side_type.1,
                        right_side_type.0,
                        right_side_type.1,
                        line_num
                    );
                }
                format!("{} = {};", left_side_cpp, right_side_cpp)
            }
        }
        MLtStatement::Normalization(matrix_name) => format!("{}.normalize();", matrix_name),
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
                expr_to_cpp(mlt_expr, ti_state, line_num, warnings)?,
                // clone ti_state here to prevent types from propagating outside the if statement
                generate_output_for_statement_list(
                    mlt_statements,
                    &mut ti_state.clone(),
                    line_num,
                    warnings
                )
            );
            text
        }
        MLtStatement::Comment(comment_str) => format!("// {}", comment_str),
        MLtStatement::Error(error_str) => {
            let _ = writeln!(warnings, "Error parsing line: {}.", error_str);
            format!("// {}; // line could not be parsed", error_str)
        }
        MLtStatement::NewLine => {
            *line_num += 1;
            format!("\n")
        }
    })
}

fn generate_output_for_statement_list(
    statement_list: Vec<MLtStatement>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> String {
    // TODO - better error context, better error types
    // TODO - better parsing errors
    // TODO - better indent
    statement_list
        .into_iter()
        .map(|s| {
            generate_output_for_statement(s, ti_state, line_num, warnings).unwrap_or_else(|e| {
                let _ = writeln!(warnings, "{}", e.0.to_string());
                format!("/* {} */", e.0.to_string())
            })
        })
        .collect()
}

fn generate_output_for_function(
    function: MLtFunction,
    ti_state: &mut HashMap<String, (u32, u32)>,
    line_num: &mut u32,
    warnings: &mut String,
) -> String {
    // TODO - infer function type from returns, handle multiple returns, etc.
    format!(
        "{} {}({}) {{{}return {};\n}}",
        type_to_cpp(*ti_state.get("_self").unwrap_or(&(0, 0))),
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
        generate_output_for_statement_list(function.body, ti_state, line_num, warnings),
        function.return_obj // TODO - type check this
    )
}

pub fn generate_eigen_output(
    file: Vec<MLtFile>,
    ti_state: &mut HashMap<String, (u32, u32)>,
    warnings: &mut String,
) -> String {
    let mut line_num = 3;
    let mut output = String::from("#include \"matlab_funcs.h\"\n\n");
    output.push_str(
        &file
            .into_iter()
            .map(|f| match f {
                MLtFile::Statement(mlt_statement) => {
                    generate_output_for_statement(mlt_statement, ti_state, &mut line_num, warnings)
                        .unwrap_or_else(|e| {
                            let _ = writeln!(warnings, "{}", e.0.to_string());
                            format!("/* {} */", e.0.to_string())
                        })
                }
                MLtFile::Function(mlt_function) => {
                    generate_output_for_function(mlt_function, ti_state, &mut line_num, warnings)
                }
            })
            .collect::<Vec<String>>()
            .join(""),
    );
    output
}
