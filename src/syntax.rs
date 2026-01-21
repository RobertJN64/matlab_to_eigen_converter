#[derive(Clone, Debug)]
pub struct MLtFunction {
    pub return_obj: String, // TODO - multiple returns?
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<MLtStatement>,
}

#[derive(Clone, Debug)]
pub enum MLtStatement {
    Assignment(MLtLValue, MLtExpr),
    Persistent(Vec<String>),                 // list of persistent variables
    IfStatement(MLtExpr, Vec<MLtStatement>), // condition, list of statements
    Comment(String),
    Error(String),
    NewLine,
    Normalization(String), // not parsed in, detected in transform pass
}

#[derive(Clone, Debug)]
pub enum MLtMatrixAccess {
    Matrix(String),                            // z
    MatrixSegment(String, MLtRange),           // z(1:3)
    MatrixMultiSegment(String, Vec<MLtRange>), // z([1:3 7:9])
    MatrixBlock(String, MLtRange, MLtRange),   // z(1:3, 4:5)
}

#[derive(Clone, Debug)]
pub enum MLtLValue {
    Integer(String), // 1 - we keep this as a string because we don't need to edit it
    Float(String),   // 0.5 - we keep this as a string because we don't need to edit it
    Matrix(MLtMatrixAccess), // `z`
    StructMatrix(String, MLtMatrixAccess), // constants.z
    InlineMatrix(Vec<MLtExpr>), // [0; 1; z]
    FunctionCall(String, Vec<MLtExpr>), // telling these from single access is impossible in matlab, list of params
}

#[derive(Clone, Debug)]
pub enum MLtExpr {
    Basic(MLtLValue), // lvalue or lvalue'
    Negation(Box<MLtExpr>),
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
    Pow,
    And,
    Or,
    EqualTo,
    NotEqualTo,
}
