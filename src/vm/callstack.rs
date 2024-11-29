use std::cell::RefCell;
use log::{trace, warn};
use super::call_frame::CallFrame;
use crate::VM;
use crate::Value;
use crate::VmError;
use crate::vm::ClassAndMethod;
use crate::vm::info;
use crate::error;
use crate::ProgramCounter;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::Reference;

pub struct CallStack<'a>{
    pub frames: Vec<CallFrame<'a>>,
    frames_infos : Vec<String>,
    current_frame: Option<RefCell<CallFrame<'a>>>
}

impl<'a> CallStack<'a> {
    pub fn new() -> Self{
        CallStack{
            frames: Vec::new(),
            frames_infos : Vec::new(),
            current_frame: None,
        }
    }

    pub fn create_call_frame<'frame>(class_and_method: ClassAndMethod<'frame>, object: Option<Reference<'frame>>, args: Vec<Value<'frame>>) -> VMResult<CallFrame<'frame>> {
        let locals = if !class_and_method.method.is_native(){
            let mut empty_locals = vec![Value::Uninitialized; class_and_method.get_max_locals()];
            for i in 0..args.len(){
                empty_locals[i] = args.get(i).unwrap().clone();
            }
            if !class_and_method.method.is_static(){
                if let Some(obj) = object {
                    empty_locals.insert(0, Value::Reference(obj));
                    empty_locals.pop();
                }
            }
            assert_eq!(empty_locals.len(), class_and_method.get_max_locals(), "Locals has not the correct length (was {}, expected {})", empty_locals.len(), class_and_method.get_max_locals());
            empty_locals
        } else {
            let mut native_locals = Vec::new();
            if !class_and_method.method.is_static() {
                if let Some(obj) = object {
                    native_locals.push(Value::Reference(obj));
                }
            }
            for i in 0..args.len(){
                native_locals.push(args.get(i).unwrap().clone());
            }
            native_locals
        };

        let args_amount = args.iter().filter(|v| **v != Value::Dummy).count();
        assert_eq!(args_amount, class_and_method.method.get_args_count(), "Args has not the correct length (was {}, expected {})", args_amount, class_and_method.method.get_args_count());
        info!("NEW CALL FRAME with {:?} locals, \nobject=({:?}), \nargs=({:?}), \nmax_locals=[{}]", locals, object, args, class_and_method.get_max_locals());

        Ok(CallFrame{
            class_and_method,
            locals,
            pc: ProgramCounter(0),
            stack: Vec::new()
        })
    }

    pub fn push_call_frame(&mut self, call_frame: CallFrame<'a>){
        //self.call_stack.push(call_frame);
        //self.call_stack.push_call_frame(call_frame);
        /*let frame_ref = unsafe {
            let frame_ptr: *const CallFrame = &mut call_frame;
            &*frame_ptr
        };*/
        //self.frames.push(frame_ref);
        //Ok(call_frame)
        //let last_frame = self.frames.last().unwrap();
        self.frames_infos.push(format!("{:?} {}", call_frame.pc, call_frame.class_and_method.format()));
        self.frames.push(call_frame);
        error!("PUSH {:?}", self.frames.last().unwrap());
    }

    pub fn pop_call_frame(&mut self) -> CallFrame<'a>{
        //self.frames.pop();
        error!("POP  {:?}", self.frames.last().unwrap());
        self.frames_infos.pop();
        self.frames.pop().unwrap()
    }
    
    pub fn add_to_top_stack(&mut self, value: Option<Value<'a>>){
        self.frames.last_mut().unwrap().prepare_reentry(value);
    }

    // Execute the last frame on the stack
    pub fn execute_top(&mut self, vm: *mut VM<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
        /*// Get a raw pointer to the last frame
        let frame_ptr: *mut CallFrame = self.frames.last_mut().unwrap();

        // SAFETY: We're handling the raw pointer correctly here. The frame is on the stack,
        // and it's valid for the duration of this method. Also, we manually manage the VM pointer.
        unsafe {
            (*frame_ptr).execute(&mut *vm)
        }*/

        /*let new_current_frame = if let Some(frame_cell) = self.current_frame.take(){
            let top = self.frames.pop().unwrap();
            self.frames.push(frame_cell.into_inner());
            top
        } else {
            self.frames.pop().unwrap()
        };*/
        //self.current_frame = Some(RefCell::new(new_current_frame));
        if self.frames.len() == 0 {
            return Ok(VMResultType::Ok(None))
        }
        unsafe {
            //self.frames.last_mut().unwrap().execute(&mut *vm)
            //let c = self.current_frame.take().unwrap();
            //let res = c.borrow_mut().execute(&mut *vm);
            trace!("execute_top popping frame for execution");
            let mut frame = self.pop_call_frame();
            let res = {
                //let frame = self.frames.last_mut().unwrap();
                frame.execute(&mut *vm)?.clone()
            };
            //let res = c.execute(&mut *vm);
            //self.frames.push(c.into_inner());
            match res {
                VMResultType::Ok(value) => {
                    //self.pop_call_frame();
                    trace!("et execution returned Ok, returning value");
                    Ok(VMResultType::Ok(value))
                },
                VMResultType::CallPaused(new_frame) => {
                    trace!("et execution returned CallPaused, returning new_frame {:?}", new_frame);
                    self.push_call_frame(frame);
                    Ok(VMResultType::CallPaused(new_frame))
                    //self.push_call_frame(frame);
                    //self.execute_top(vm)
                }
                VMResultType::ExceptionThrown(error, throwable) => {
                    trace!("et execution returned ExceptionThrown, returning frame {:?} and error {:?}", frame, error);
                    //self.push_call_frame(frame);
                    Ok(VMResultType::ExceptionThrown(error, throwable))
                }
            }
            //Ok(res)
        }
    }

    pub fn print_call_stack(&self) {
        for (index, call_frame_info) in self.frames_infos.iter().enumerate(){
            //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
            warn!("[{}]: {}", index, call_frame_info);
        }
    }
}