use crate::syntax::*;

fn transform_statement(statement: MLtStatement) -> MLtStatement {
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
            body.into_iter().map(transform_statement).collect(),
        );
    }

    statement
}

pub fn transform_ast(mut function: MLtFunction) -> MLtFunction {
    function.body = function.body.into_iter().map(transform_statement).collect();

    function
}
