use std::fmt;
use std::fmt::Formatter;

use thiserror::Error;

use crate::vm::class_path_entry::ClassPathEntryResolveError;

/// Error returned when a directory is not valid
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidDirectoryError {
    pub(crate) path: String,
}

impl fmt::Display for InvalidDirectoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid directory: {}", self.path)
    }
}

impl std::error::Error for InvalidDirectoryError {}

#[derive(Error, Debug)]
pub enum ClassParseError{
    #[error("Reached EOF when parsing a class")]
    ReadError,
    #[error("Classpath Entry Resolve failed: {0}")]
    EntryResolveError(#[from] ClassPathEntryResolveError),
    #[error("Class Resolve Error: {0}")]
    ClassResolveError(String),
    #[error("{0}")]
    ConstantPoolError(String),
}