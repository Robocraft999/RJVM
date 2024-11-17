use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::error;
use thiserror::Error;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::error::{ClassParseError, InvalidDirectoryError};
use crate::vm::java_error::JavaError;
use crate::vm::VmError;

pub trait ClassPathEntry : fmt::Debug{
    fn resolve(&self, class_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError>;
    fn resolve_file(&self, file_name: &str) -> Result<Option<Vec<u8>>, VmError>;
}

#[derive(Debug)]
pub struct FileSystemClassPathEntry {
    pub file_dir: PathBuf
}

impl FileSystemClassPathEntry{
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, InvalidDirectoryError>{
        let mut dir = PathBuf::new();
        dir.push(path);
        if !dir.exists() || !dir.is_dir(){
            Err(InvalidDirectoryError{path: dir.to_string_lossy().to_string()})
        } else {
            Ok(Self{file_dir: dir})
        }
    }
}

impl ClassPathEntry for FileSystemClassPathEntry{
    fn resolve(&self, class_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError>{
        let mut candidate = self.file_dir.clone();
        let class_name = class_name.replace(".", "/");
        candidate.push(class_name);
        candidate.set_extension("class");
        if candidate.exists(){
            std::fs::read(candidate)
                .map(Some)
                .map_err(From::from)
        } else {
            Ok(None)
        }
    }

    fn resolve_file(&self, file_name: &str) -> Result<Option<Vec<u8>>, VmError> {
        let mut candidate = self.file_dir.clone();
        candidate.push(file_name);
        if candidate.exists(){
            std::fs::read(candidate)
                .map(Some)
                .map_err(|e| VmError::JavaException(JavaError::IOException(e.to_string())))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct JarClassPathEntry{
    file_name: String,
    archive: RefCell<ZipArchive<File>>
}

impl JarClassPathEntry{
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Result<Self, JarFileError>{
        let path = path.as_ref();
        let file_name = path.to_string_lossy().to_string();
        if !path.exists(){
            return Err(JarFileError::NotFound(file_name))
        }
        let file = File::open(path).map_err(|_| JarFileError::ReadingError(file_name.to_string()))?;
        let archive = ZipArchive::new(file).map_err(|_| JarFileError::InvalidJar(file_name.to_string()))?;

        Ok(Self{
            file_name,
            archive: RefCell::new(archive)
        })
    }
}

impl ClassPathEntry for JarClassPathEntry{
    fn resolve(&self, class_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError>{
        let class_file_name = class_name.replace(".", "/").to_string() + ".class";
        self.resolve_file(class_file_name.as_str()).map_err(|e| ClassLoadingError::from(std::io::Error::last_os_error()))
    }

    fn resolve_file(&self, file_name: &str) -> Result<Option<Vec<u8>>, VmError> {
        match self.archive.borrow_mut().by_name(file_name) {
            Ok(mut zip) => {
                let mut buffer: Vec<u8> = Vec::with_capacity(zip.size() as usize);
                zip
                    .read_to_end(&mut buffer)
                    .map_err(|e| VmError::JavaException(JavaError::IOException(e.to_string())))?;
                Ok(Some(buffer))
            }
            Err(err) => match err{
                ZipError::FileNotFound => Ok(None),
                _ => Err(VmError::ParseError(ClassParseError::LoadingError(ClassLoadingError::from(err)))),
            }
        }
    }
}

#[derive(Debug)]
pub struct ClassLoadingErrorr {
    message: String,
    source: Box<dyn Error>,
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum ClassLoadingError {
    #[error("IOError: {0}")]
    IOError(String),
    #[error("ZipError: {0}")]
    ZipError(String)
}

impl From<ZipError> for ClassLoadingError{
    fn from(value: ZipError) -> Self {
        ClassLoadingError::ZipError(value.to_string())
    }
}

impl From<std::io::Error> for ClassLoadingError{
    fn from(value: std::io::Error) -> Self {
        ClassLoadingError::IOError(value.to_string())
    }
}

/// Error returned if searching a class inside a Jar fails
#[derive(Error, Debug, PartialEq, Clone)]
pub enum JarFileError {
    /// The jar file does not exist!
    #[error("file {0} not found")]
    NotFound(String),

    /// Generic I/O error reading the file
    #[error("error reading file {0}")]
    ReadingError(String),

    /// The file is not actually a valid jar
    #[error("file {0} is not a valid jar")]
    InvalidJar(String),
}