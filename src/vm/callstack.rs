use super::call_frame::CallFrame;
use crate::VM;
use crate::Value;
use crate::VmError;
use crate::error;
use std::{borrow::BorrowMut, cell::RefCell};

pub struct CallStack<'a>{
    frames: RefCell<Vec<CallFrame<'a>>>
}

impl<'a> CallStack<'a> {
    pub fn new() -> Self{
        CallStack{
            frames: RefCell::new(Vec::new())
        }
    }

    pub fn push_call_frame(&self, frame: CallFrame<'a>){
        self.frames.borrow_mut().push(frame)
    }

    pub fn execute(&self, vm: &mut VM<'a>) -> Result<Option<Value<'a>>, VmError>{
        self.frames.borrow_mut().last_mut().unwrap().execute(vm)
    }

    pub fn pop_call_frame(&self){
        self.frames.borrow_mut().pop();
    }

    pub fn print_call_stack(&self) {
        for (index, call_frame) in self.frames.borrow().iter().enumerate(){
            error!("[{}]: {:?}", index, call_frame);
        }
    }
}