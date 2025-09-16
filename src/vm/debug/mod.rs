use crate::vm::debug::exceptions::ExceptionHelper;

mod exceptions;

pub struct DebugHelper{
    pub exception_helper: ExceptionHelper,
}

impl DebugHelper{
    pub fn new() -> Self{
        Self{
            exception_helper: ExceptionHelper::new()
        }
    }
}