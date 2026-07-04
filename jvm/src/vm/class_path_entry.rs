use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::error::{ClassParseError, InvalidDirectoryError};
use crate::vm::java_error::JavaError;
use crate::vm::result::VMResult;
use crate::vm::VmError;

#[derive(Debug)]
pub enum ClassPathEntry {
    FileSystem(PathBuf),
    Jar(Mutex<ZipArchive<File>>)
}

impl ClassPathEntry {
    pub fn new(string: &str) -> VMResult<Self> {
        Self::try_new_dir(string).or(Self::try_new_jar(string)).map_err(|e| VmError::ParseError(ClassParseError::EntryResolveError(e.into())))
    }

    fn try_new_dir<P: AsRef<Path>>(path: P) -> Result<Self, InvalidDirectoryError>{
        let mut dir = PathBuf::new();
        dir.push(path);
        if !dir.exists() || !dir.is_dir(){
            let path = std::path::absolute(&dir).map(|p| p.to_string_lossy().to_string()).unwrap_or(dir.to_string_lossy().to_string());
            Err(InvalidDirectoryError{path})
        } else {
            Ok(ClassPathEntry::FileSystem(dir))
        }
    }

    fn try_new_jar<P: AsRef<Path>>(path: P) -> Result<Self, JarFileError>{
        let path = path.as_ref();
        let file_name = path.to_string_lossy().to_string();
        if !path.exists(){
            return Err(JarFileError::NotFound(file_name))
        }
        let file = File::open(path).map_err(|_| JarFileError::ReadingError(file_name.to_string()))?;
        let archive = ZipArchive::new(file).map_err(|_| JarFileError::InvalidJar(file_name.to_string()))?;

        Ok(ClassPathEntry::Jar(Mutex::new(archive)))
    }

    pub fn resolve(&self, class_name: &str) -> VMResult<Option<Vec<u8>>>{
        match self {
            ClassPathEntry::FileSystem(path) => {
                let mut candidate = path.clone();
                let class_name = class_name.replace(".", "/");
                candidate.push(class_name);
                candidate.set_extension("class");
                if candidate.exists(){
                    std::fs::read(candidate)
                        .map(Some)
                        .map_err(|e| VmError::ParseError(ClassParseError::EntryResolveError(e.into())))
                } else {
                    Ok(None)
                }
            }
            ClassPathEntry::Jar(..) => {
                let class_file_name = class_name.replace(".", "/").to_string() + ".class";
                self.resolve_file(class_file_name.as_str())
            }
        }
    }

    pub fn resolve_file(&self, file_name: &str) -> VMResult<Option<Vec<u8>>> {
        match self {
            ClassPathEntry::FileSystem(path) => {
                let mut candidate = path.clone();
                candidate.push(file_name);
                if candidate.exists(){
                    std::fs::read(candidate)
                        .map(Some)
                        .map_err(|e| VmError::JavaException(JavaError::IOException(e.to_string())))
                } else {
                    Ok(None)
                }
            }
            ClassPathEntry::Jar(archive) => {
                match archive.lock()?.by_name(file_name) {
                    Ok(mut zip) => {
                        let mut buffer: Vec<u8> = Vec::with_capacity(zip.size() as usize);
                        zip
                            .read_to_end(&mut buffer)
                            .map_err(|e| VmError::ParseError(ClassParseError::EntryResolveError(e.into())))?;
                        Ok(Some(buffer))
                    }
                    Err(err) => match err{
                        ZipError::FileNotFound => Ok(None),
                        _ => Err(VmError::ParseError(ClassParseError::EntryResolveError(ClassPathEntryResolveError::from(err)))),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ClassPathEntryResolveError {
    #[error("IOError: {0}")]
    IOError(#[from] std::io::Error),
    #[error("ZipError: {0}")]
    ZipError(String),
    #[error("JarError: {0}")]
    JarFileError(#[from] JarFileError)
}

impl From<ZipError> for ClassPathEntryResolveError {
    fn from(value: ZipError) -> Self {
        ClassPathEntryResolveError::ZipError(value.to_string())
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