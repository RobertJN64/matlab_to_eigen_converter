#[derive(Clone, Debug)]
pub struct MLtFunction {
    pub return_obj: String,
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<MLtStatement>,
}

#[derive(Clone, Debug)]
pub enum MLtStatement {
    Assignment(MLtAssignment),
    IfStatement(MLtExpr, Vec<MLtStatement>), // condition, list of statements
    Comment(String),
    Error(String),
}

#[derive(Clone, Debug)]
// matches `lvalue = expr;`
pub struct MLtAssignment {
    pub lvalue: MLtLValue,
    pub expr: MLtExpr,
}

#[derive(Clone, Debug)]
pub enum MLtLValue {
    // TODO - do we need 1 and 2d access?
    Integer(String), // 1 - we keep this as a string because we don't need to edit it
    Float((String, String)), // 0.5 - we keep this as a string because we don't need to edit it
    Matrix(String),  // `z`
    MatrixSegment(String, MLtRange), // `z(1:3)`
    MatrixBlock(String, MLtRange, MLtRange), // `z(1:3, 2:4)`
    FunctionCall(String, Vec<MLtExpr>), // telling these from single access is impossible in matlab, list of params
}

#[derive(Clone, Debug)]
pub enum MLtExpr {
    Basic(MLtLValue),                            // lvalue
    BinOp(Box<MLtExpr>, MLtBinOp, Box<MLtExpr>), // "lvalue + lvalue", or sub, mul, div
}

#[derive(Clone, Debug)]
pub struct MLtRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug)]
pub enum MLtBinOp {
    Add,
    Sub,
    Mul,
    Div,
}
