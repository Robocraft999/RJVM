use crate::vm::call_frame::CallFrame;
use crate::vm::value::Value;
use crate::vm::VmError;

pub type VMPartialResult<'a, T> = Result<VMResultType<'a, T>, VmError>;
pub type VMResult<T> = Result<T, VmError>;

#[derive(Debug, Clone)]
pub enum VMResultType<'f, T> {
    Ok(T),
    CallPaused(CallFrame<'f>),
    ExceptionThrown(VmError, Value<'f>),
    NeedsClassInit(Vec<CallFrame<'f>>)
}

impl<'a, T> VMResultType<'a, T> {
    /*pub fn to_result(self) -> VMResult<T> {
        Ok(self)
    }*/

    pub fn is_ok(&self) -> bool {
        match self {
            VMResultType::Ok(..) => true,
            _ => false
        }
    }
    
    pub fn is_call_paused(&self) -> bool {
        match self { 
            VMResultType::CallPaused(..) => true,
            _ => false
        }
    }
}

impl<'o, T> VMResultType<'o, Option<T>> {
    pub fn to_option(self) -> Option<T>{
        match self {
            VMResultType::Ok(value) => value,
            _ => None
        }
    }
}

/*impl<'a, T> From<VMResultType<'a, T>> for Result<T, VmError> {
    fn from(result: VMResultType<T>) -> Self {
        result.to_result()
    }
}*/