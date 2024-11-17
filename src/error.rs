use std::fmt;
use std::fmt::Formatter;

use thiserror::Error;

use crate::vm::class_path_entry::ClassLoadingError;

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

#[derive(Error, Debug, PartialEq, Clone)]
pub enum ClassParseError{
    #[error("Reached EOF when parsing a class")]
    ReadError,
    #[error("Class loading failed")]
    LoadingError(#[from] ClassLoadingError),
    #[error("Could not resolve class {0}")]
    ResolveError(String),
}