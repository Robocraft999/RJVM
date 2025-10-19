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
    pub frames: RefCell<Vec<CallFrame<'a>>>,
    pub operand_stacks: RefCell<Vec<Vec<Value<'a>>>>,
    pub locals_stack: RefCell<Vec<Vec<Value<'a>>>>,
    pub pcs: RefCell<Vec<ProgramCounter>>,
}

impl<'a> CallStack<'a> {
    pub fn new() -> Self{
        CallStack{
            frames: RefCell::new(Vec::new()),
            operand_stacks: RefCell::new(Vec::new()),
            locals_stack: RefCell::new(Vec::new()),
            pcs: RefCell::new(Vec::new()),
        }
    }

    pub fn create_and_push_call_frame(&self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>, should_push_return: bool){
        let mut locals = vec![Value::Uninitialized; class_and_method.get_max_locals() + if class_and_method.method.is_static() {1} else {0}];
        let mut offset = 0;
        if !class_and_method.method.is_static(){
            locals[0] = Value::Reference(object.unwrap());
            offset = 1;
        }
        for (dest, src) in locals[offset..].iter_mut().zip(args) {
            *dest = src;
        }
        self.locals_stack.borrow_mut().push(locals);
        self.operand_stacks.borrow_mut().push(Vec::with_capacity(class_and_method.get_max_stack_size()));
        self.pcs.borrow_mut().push(ProgramCounter(0));
        let frame = CallFrame{
            class_and_method,
            should_push_return,
        };
        self.frames.borrow_mut().push(frame);
    }

    pub fn pop_call_frame(&self) -> CallFrame<'a>{
        self.locals_stack.borrow_mut().pop();
        self.operand_stacks.borrow_mut().pop();
        self.pcs.borrow_mut().pop();
        self.frames.borrow_mut().pop().unwrap()
    }

    pub fn pop_call_frame_at(&self, index: usize) -> CallFrame<'a>{
        if index == self.frames.borrow().len() - 1{
            self.pop_call_frame()
        } else {
            self.locals_stack.borrow_mut().remove(index);
            self.operand_stacks.borrow_mut().remove(index);
            self.pcs.borrow_mut().remove(index);
            self.frames.borrow_mut().remove(index)
        }
    }

    pub fn push_operand_value(&self, val: Value<'a>){
        if self.operand_stacks.borrow().last().unwrap().len() == self.operand_stacks.borrow().last().unwrap().capacity(){
            panic!("Method Stack overflown");
        }
        self.operand_stacks.borrow_mut().last_mut().unwrap().push(val);
    }

    pub fn pop_operand_value(&self) -> Option<Value<'a>>{
        self.operand_stacks.borrow_mut().last_mut().unwrap().pop() //TODO make it VMResult and add error type
    }

    pub fn store_local(&self, val: Value<'a>, index: usize){
        self.locals_stack.borrow_mut().last_mut().unwrap()[index] = val;
    }

    pub fn load_local(&self, index: usize) -> Option<Value<'a>>{
        self.locals_stack.borrow().last().unwrap().get(index).cloned() //TODO same as above
    }

    pub fn set_pc(&self, val: u16){
        *self.pcs.borrow_mut().last_mut().unwrap() = ProgramCounter(val);
    }

    pub fn get_pc(&self) -> ProgramCounter{
        *self.pcs.borrow().last().unwrap()
    }

    pub fn get_class_and_method_cloned(&self) -> ClassAndMethod<'a> {
        self.frames.borrow().last().unwrap().class_and_method.clone()
    }

    pub fn len(&self) -> usize{
        self.frames.borrow().len()
    }

    pub fn is_empty(&self) -> bool{
        self.frames.borrow().is_empty()
    }

    pub fn print_call_stack(&self) {
        for (index, call_frame_info) in self.frames.borrow().iter().enumerate(){
            //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
            warn!("[{}]: {:?}", index, self.format_frame(index, &call_frame_info.class_and_method));
        }
    }

    fn format_frame(&self, index: usize, cam: &ClassAndMethod) -> String{
        let mut line_number = -1;
        let mut instruction = None;
        let pc = self.pcs.borrow()[index].0;
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