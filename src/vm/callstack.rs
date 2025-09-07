use std::cell::RefCell;
use log::{trace, warn};
use crate::attribute::{Code, ExceptionTable, VisibleRuntimeAnnotations};
use crate::bytecode::Instruction;
use crate::constants::ConstantPool;
use super::call_frame::CallFrame;
use crate::VM;
use crate::Value;
use crate::VmError;
use crate::vm::ClassAndMethod;
use crate::vm::info;
use crate::error;
use crate::field_info::FieldType;
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::ProgramCounter;
use crate::vm::class::{Class, ClassId, ClassRef};
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::Reference;

pub struct CallStack<'a>{
    pub frames: Vec<CallFrame<'a>>,
    pub current_frame: Option<ClassAndMethod<'a>>,
    pub operand_stacks: Vec<Vec<Value<'a>>>,
    pub locals_stack: Vec<Vec<Value<'a>>>,
    pub pcs: Vec<ProgramCounter>,
}

impl<'a> CallStack<'a> {
    pub fn new() -> Self{
        CallStack{
            frames: Vec::new(),
            current_frame: None,
            operand_stacks: Vec::new(),
            locals_stack: Vec::new(),
            pcs: Vec::new(),
        }
    }

    /*pub fn create_returning_frame<'frame>(class: ClassRef<'frame>, object: Value<'frame>) -> CallFrame<'frame> {
        let method = &class.methods[0];
        let class_and_method = ClassAndMethod {
            class,
            method,
        };
        let locals = Vec::new();
        CallFrame{
            class_and_method,
            locals,
            pc: ProgramCounter(0),
            last_pc: ProgramCounter(0),
            stack: vec![object]
        }
    }
    
    pub fn create_throwing_frame<'frame>(class: ClassRef<'frame>, object: Value<'frame>) -> CallFrame<'frame> {
        let method = &class.methods[1];
        let class_and_method = ClassAndMethod {
            class,
            method,
        };
        let locals = Vec::new();
        CallFrame{
            class_and_method,
            locals,
            pc: ProgramCounter(0),
            last_pc: ProgramCounter(0),
            stack: vec![object]
        }
    }*/

    /*pub fn create_call_frame_old<'frame>(class_and_method: ClassAndMethod<'frame>, object: Option<Reference<'frame>>, args: Vec<Value<'frame>>) -> CallFrame<'frame> {
        let mut offset = 0;
        for (i, arg) in class_and_method.method.descriptor.args.iter().enumerate() {
            let mut arg1 = args.get(i + offset).unwrap();
            if arg1 == &Value::Dummy {
                offset += 1;
                arg1 = args.get(i + offset).unwrap();
            }
            if let FieldType::Object(class_name) = arg{
                if arg1 == &Value::Null{
                    continue
                }
                if let Value::Reference(reference) = arg1{
                    /*if &reference.class_name != class_name{
                        panic!("Wrong class name!: {}, expected: {}", reference.class_name, class_name);
                    }*/
                } else {
                    panic!("Wrong argument type for {} [{}, off({})]!: {:?}, expected: {}\nfull args: {:?}", class_and_method.format(), i, offset, arg1, class_name, args);
                }
            }
        }
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

        CallFrame{
            class_and_method,
            locals,
            pc: ProgramCounter(0),
            last_pc: ProgramCounter(0),
            stack: Vec::new()
        }
    }*/

    pub fn create_and_push_call_frame(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>){
        let mut locals = vec![Value::Uninitialized; class_and_method.get_max_locals()];
        let mut offset = 0;
        if !class_and_method.method.is_static(){
            locals[0] = Value::Reference(object.unwrap());
            offset = 1;
        }
        for (dest, src) in locals[offset..].iter_mut().zip(args){
            *dest = src;
        }
        self.locals_stack.push(locals);
        self.operand_stacks.push(Vec::with_capacity(class_and_method.get_max_stack_size()));
        self.pcs.push(ProgramCounter(0));
        let frame = CallFrame{
            class_and_method
        };
        self.frames.push(frame);
    }

    /*pub fn push_call_frame(&mut self, call_frame: CallFrame<'a>){
        //self.call_stack.push(call_frame);
        //self.call_stack.push_call_frame(call_frame);
        /*let frame_ref = unsafe {
            let frame_ptr: *const CallFrame = &mut call_frame;
            &*frame_ptr
        };*/
        //self.frames.push(frame_ref);
        //Ok(call_frame)
        //let last_frame = self.frames.last().unwrap();
        self.frames.push(call_frame);
        info!("PUSH {:?}", self.frames.last().unwrap());
        if self.frames.len() > 200{
            for (index, call_frame_info) in self.frames.iter().enumerate(){
                //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
                println!("[{}]: {:?}", index, call_frame_info);
            }
            panic!("Stack overflow");
        }
    }*/

    pub fn pop_call_frame(&mut self) -> CallFrame<'a>{
        self.locals_stack.pop();
        self.operand_stacks.pop();
        self.pcs.pop();
        self.frames.pop().unwrap()
    }

    /*pub fn pop_call_frame_old(&mut self) -> CallFrame<'a>{
        //self.frames.pop();
        info!("POP  {:?}", self.frames.last().unwrap());
        self.frames.pop().unwrap()
    }*/
    
    /*pub fn add_to_top_stack(&mut self, value: Option<Value<'a>>){
        self.frames.last_mut().unwrap().prepare_reentry(value);
    }*/

    pub fn push_operand_value(&mut self, val: Value<'a>){
        if self.operand_stacks.last().unwrap().len() == self.operand_stacks.last().unwrap().capacity(){
            panic!("Method Stack overflown");
        }
        self.operand_stacks.last_mut().unwrap().push(val);
    }

    pub fn pop_operand_value(&mut self) -> Option<Value<'a>>{
        self.operand_stacks.last_mut().unwrap().pop() //TODO make it VMResult and add error type
    }

    pub fn store_local(&mut self, val: Value<'a>, index: usize){
        self.locals_stack.last_mut().unwrap()[index] = val;
    }

    pub fn load_local(&self, index: usize) -> Option<Value<'a>>{
        self.locals_stack.last().unwrap().get(index).cloned() //TODO same as above
    }

    pub fn set_pc(&mut self, val: u16){
        *self.pcs.last_mut().unwrap() = ProgramCounter(val);
    }

    pub fn get_pc(&self) -> ProgramCounter{
        *self.pcs.last().unwrap()
    }

    // Execute the last frame on the stack
    /*pub fn execute_top(&mut self, vm: *mut VM<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
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
            self.current_frame = Some(frame.class_and_method.clone());
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
                    self.push_call_frame(frame);
                    Ok(VMResultType::ExceptionThrown(error, throwable))
                }
                VMResultType::NeedsClassInit(frames, reenter) => {
                    trace!("et execution returned NeedsClassInit, returning frames {:?}", frames);
                    frame.pc = frame.last_pc.clone();
                    if reenter {
                        self.push_call_frame(frame);
                    }
                    Ok(VMResultType::NeedsClassInit(frames, reenter))
                }
            }
            //Ok(res)
        }
    }*/

    pub fn print_call_stack(&self) {
        for (index, call_frame_info) in self.frames.iter().enumerate(){
            //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
            warn!("[{}]: {:?}", index, self.format_frame(index, &call_frame_info.class_and_method));
        }
    }

    fn format_frame(&self, index: usize, cam: &ClassAndMethod) -> String{
        let mut line_number = -1;
        let mut instruction = None;
        let pc = self.pcs[index].0;
        if let Some(code) = &cam.method.code{
            instruction = cam.method.code_blocks.as_ref().map(|blocks| blocks.get(&pc).unwrap());
            if let Some(line_number_table) = &code.line_number_table{
                for entry in line_number_table.0.iter().rev(){
                    if entry.program_counter.0 < pc || (pc == 0 && entry.program_counter.0 == 0) {
                        line_number = entry.line_number.0 as i32;
                        break; 
                    }
                }
            }
        };
        format!("Method: {}:{} at {:?} ({:?})", cam.format(), line_number, pc, instruction)
    }
}