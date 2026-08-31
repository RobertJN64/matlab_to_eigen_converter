// TODO - unit tests

#[derive(Clone, Debug)]
pub enum MLtFile {
    Statement(MLtStatement),
    Function(MLtFunction),
}

#[derive(Clone, Debug)]
pub struct MLtFunction {
    pub return_obj: String, // TODO - multiple returns?
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<MLtStatement>,
}

#[derive(Clone, Debug)]
pub enum MLtStatement {
    Expression(MLtExpr),
    Assignment(MLtValue, MLtExpr),
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
    MatrixIndex(String, u32), // z(1) - this is impossible to tell from a function call during parsing so we catch it as a transform
    MatrixSegment(String, MLtRange), // z(1:3)
    MatrixMultiSegment(String, Vec<MLtRange>), // z([1:3 7:9])
    MatrixBlock(String, MLtRange, MLtRange), // z(1:3, 4:5)
}

#[derive(Clone, Debug)]
pub enum MLtValue {
    Integer(u32), // 1 - converted to an integer for ease of code generation and type checking
    Float(String), // 0.5 - we keep this as a string because we don't need to edit it
    Matrix(MLtMatrixAccess), // `z`
    StructMatrix(String, MLtMatrixAccess), // constants.z
    InlineMatrix(Vec<MLtExpr>), // [0; 1; z]
    FunctionCall(String, Vec<MLtExpr>), // telling these from single access is impossible in matlab, list of params
}

#[derive(Clone, Debug)]
pub enum MLtExpr {
    Basic(MLtValue),                             // base pattern, a value
    Negation(Box<MLtExpr>),                      // -expr
    Transposed(Box<MLtExpr>),                    // expr'
    Parenthesized(Box<MLtExpr>),                 // (expr)
    BinOp(Box<MLtExpr>, MLtBinOp, Box<MLtExpr>), // "expr + expr", or sub, mul, div
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
    CwiseMul, // component/element wise mul
    Div,
    CwiseDiv, // component/element wise div
    Pow,
    CwisePow, // component/element wise pow
    And,
    Or,
    EqualTo,
    NotEqualTo,
    LessThan,
    LessThanEqualTo,
    GreaterThan,
    GreaterThanEqualTo,
}
