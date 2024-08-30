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

use crate::error::InvalidDirectoryError;

pub trait ClassPathEntry : fmt::Debug{
    fn resolve(&self, class_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError>;
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
        candidate.push(class_name);
        candidate.set_extension("class");
        if candidate.exists(){
            std::fs::read(candidate)
                .map(Some)
                .map_err(ClassLoadingError::new)
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
        let class_file_name = class_name.to_string() + ".class";
        match self.archive.borrow_mut().by_name(class_file_name.as_str()) {
            Ok(mut zip) => {
                let mut buffer: Vec<u8> = Vec::with_capacity(zip.size() as usize);
                zip
                    .read_to_end(&mut buffer)
                    .map_err(ClassLoadingError::new)?;
                Ok(Some(buffer))
            }
            Err(err) => match err{
                ZipError::FileNotFound => Ok(None),
                _ => Err(ClassLoadingError::new(err)),
            }
        }
    }
}

#[derive(Debug)]
pub struct ClassLoadingError {
    message: String,
    source: Box<dyn Error>,
}

impl PartialEq for ClassLoadingError{
    fn eq(&self, other: &Self) -> bool {
        self.message.eq(&other.message)
    }
}

impl ClassLoadingError {
    pub fn new(error: impl Error + 'static) -> Self {
        Self {
            message: error.to_string(),
            source: Box::new(error),
        }
    }
}

impl Display for ClassLoadingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ClassLoadingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Error returned if searching a class inside a Jar fails
#[derive(Error, Debug, PartialEq)]
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