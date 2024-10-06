use thiserror::Error;

use crate::vm::class_path_entry::{ClassLoadingError, ClassPathEntry, FileSystemClassPathEntry, JarClassPathEntry};
use crate::vm::VmError;

#[derive(Debug, Default)]
pub struct ClassPath{
    entries: Vec<Box<dyn ClassPathEntry>>
}

#[derive(Error, Debug, PartialEq)]
pub enum ClassPathParseError{
    #[error("invalid classpath entry {0}")]
    InvalidEntry(String),
}

impl ClassPath{
    pub fn push(&mut self, string: &str) -> Result<(), ClassPathParseError>{
        let mut entries_to_add = Vec::new();
        for entry in string.split(";"){
            let parsed_entry = self.try_parse_entry(entry)?;
            entries_to_add.push(parsed_entry);
        }
        self.entries.append(&mut entries_to_add);
        Ok(())
    }

    fn try_parse_entry(&self, string: &str) -> Result<Box<dyn ClassPathEntry>, ClassPathParseError> {
        self.try_parse_entry_as_dir(string).or(self.try_parse_entry_as_jar(string))
    }

    fn try_parse_entry_as_dir(&self, string: &str) -> Result<Box<dyn ClassPathEntry>, ClassPathParseError>{
        let entry = FileSystemClassPathEntry::new(string).map_err(|_| ClassPathParseError::InvalidEntry(string.to_string()))?;
        Ok(Box::new(entry))
    }

    fn try_parse_entry_as_jar(&self, string: &str) -> Result<Box<dyn ClassPathEntry>, ClassPathParseError>{
        let entry = JarClassPathEntry::new(string).map_err(|_| ClassPathParseError::InvalidEntry(string.to_string()))?;
        Ok(Box::new(entry))
    }

    pub fn resolve(&self, class_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError>{
        for entry in self.entries.iter() {
            let entry_result = entry.resolve(class_name)?;
            if let Some(class_bytes) = entry_result{
                return Ok(Some(class_bytes))
            }
        }
        Ok(None)
    }

    pub fn resolve_file(&self, file_name: &str) -> Result<Option<Vec<u8>>, VmError>{
        for entry in self.entries.iter() {
            let entry_result = entry.resolve_file(file_name)?;
            if let Some(class_bytes) = entry_result{
                return Ok(Some(class_bytes))
            }
        }
        Ok(None)
    }
}

