pub struct TranspilerError(pub String);

pub struct InvalidType(pub String);

impl From<InvalidType> for TranspilerError {
    fn from(err: InvalidType) -> Self {
        TranspilerError(format!("Invalid type: <{}>", err.0))
    }
}

impl std::fmt::Display for TranspilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
