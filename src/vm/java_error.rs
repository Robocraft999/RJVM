use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum JavaError{
    #[error("Class not found: {0}")]
    ClassNotFoundException(String),
    #[error("Class not found: {0}")]
    MethodNotFoundException(String),
    #[error("{0}: {1}")]
    JavaExceptionThrown(String, String),
    #[error("Division by zero error")]
    DivisionByZero,
    #[error("NullPointerException {0}")]
    NullPointerException(String),
}