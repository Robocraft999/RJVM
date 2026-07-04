use thiserror::Error;

use crate::vm::class_path_entry::ClassPathEntry;
use crate::vm::result::VMResult;

#[derive(Debug, Default)]
pub struct ClassPath{
    entries: Vec<ClassPathEntry>
}

#[derive(Error, Debug, PartialEq)]
pub enum ClassPathParseError{
    #[error("invalid classpath entry {0}")]
    InvalidEntry(String),
}

impl ClassPath{
    pub fn push(&mut self, string: &str) -> VMResult<()>{
        let mut entries_to_add = Vec::new();
        for entry in string.split(";"){
            let parsed_entry = ClassPathEntry::new(entry)?;
            entries_to_add.push(parsed_entry);
        }
        self.entries.append(&mut entries_to_add);
        Ok(())
    }

    pub fn resolve(&self, class_name: &str) -> VMResult<Option<Vec<u8>>>{
        for entry in self.entries.iter() {
            let entry_result = entry.resolve(class_name)?;
            if let Some(class_bytes) = entry_result{
                return Ok(Some(class_bytes))
            }
        }
        Ok(None)
    }

    pub fn resolve_file(&self, file_name: &str) -> VMResult<Option<Vec<u8>>>{
        for entry in self.entries.iter() {
            let entry_result = entry.resolve_file(file_name)?;
            if let Some(class_bytes) = entry_result{
                return Ok(Some(class_bytes))
            }
        }
        Ok(None)
    }
}

