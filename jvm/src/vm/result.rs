use crate::vm::value::Value;
use crate::vm::VmError;

pub type VMPartialResult<T> = Result<VMResultType<T>, VmError>;
pub type VMResult<T> = Result<T, VmError>;

#[derive(Debug, Clone)]
pub enum VMResultType<T> {
    Successful(T),
    ExceptionThrown,
    Interrupted(usize, bool)
}