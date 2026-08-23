use crate::vm::VmError;

pub type VMPartialResult<T> = Result<VMResultType<T>, VmError>;
pub type VMResult<T> = Result<T, VmError>;

#[derive(Debug, Clone)]
pub enum VMResultType<T> {
    Successful(T),
    ExceptionThrown,
    Interrupted(usize)
}

impl <T> VMResultType<T>{
    pub fn map<U, F>(self, f: F) -> VMResultType<U> 
    where
        F: FnOnce(T) -> U
    {
        match self {
            VMResultType::Successful(t) => VMResultType::Successful(f(t)),
            VMResultType::ExceptionThrown => VMResultType::ExceptionThrown,
            VMResultType::Interrupted(t) => VMResultType::Interrupted(t)
        }
    }
    
    pub fn to_result(self) -> VMPartialResult<T> {
        Ok(self)
    }
}