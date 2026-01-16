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
    Persistent(Vec<String>),                 // list of persistent variables
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
pub enum MLtMatrixAccess {
    Matrix(String),
    MatrixSegment(String, MLtRange),
    MatrixBlock(String, MLtRange, MLtRange),
}

#[derive(Clone, Debug)]
pub enum MLtLValue {
    // TODO - consider breaking into postfix
    Integer(String), // 1 - we keep this as a string because we don't need to edit it
    Float(String, String), // 0.5 - we keep this as a string because we don't need to edit it
    Matrix(MLtMatrixAccess), // `z`
    StructMatrix(String, MLtMatrixAccess), // constants.z
    InlineMatrix(Vec<MLtExpr>), // [0; 1; z]
    FunctionCall(String, Vec<MLtExpr>), // telling these from single access is impossible in matlab, list of params
}

#[derive(Clone, Debug)]
pub enum MLtExpr {
    Basic(MLtLValue),         // lvalue or lvalue'
    Transposed(Box<MLtExpr>), // transposed will be parenthesized or lvalue
    Parenthesized(Box<MLtExpr>),
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
    NotEqualTo,
}
