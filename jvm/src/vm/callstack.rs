use super::call_frame::CallFrame;
use crate::vm::class::ClassAndMethodId;
use crate::vm::value::Reference;
use crate::vm::ClassAndMethod;
use crate::vm::ProgramCounter;
use crate::Value;
use crate::VM;
use log::{trace, warn};
use std::cell::RefCell;
use crate::vm::debug::validation::FieldTypeExt;

pub struct CallStack{
    pub frames: RefCell<Vec<CallFrame>>,
    pub operand_stacks: RefCell<Vec<Vec<Value>>>,
    pub locals_stack: RefCell<Vec<Vec<Value>>>,
    pub pcs: RefCell<Vec<ProgramCounter>>,
}

impl CallStack {
    pub fn new() -> Self{
        CallStack{
            frames: RefCell::new(Vec::new()),
            operand_stacks: RefCell::new(Vec::new()),
            locals_stack: RefCell::new(Vec::new()),
            pcs: RefCell::new(Vec::new()),
        }
    }

    pub fn pop_call_frame(&self) -> CallFrame{
        self.locals_stack.borrow_mut().pop();
        self.operand_stacks.borrow_mut().pop();
        self.pcs.borrow_mut().pop();
        trace!("Popping frame for: {:?}", self.frames.borrow().last().unwrap().class_and_method);
        self.frames.borrow_mut().pop().unwrap()
    }

    pub fn pop_call_frame_at(&self, index: usize) -> CallFrame{
        if index == self.frames.borrow().len() - 1{
            self.pop_call_frame()
        } else {
            self.locals_stack.borrow_mut().remove(index);
            self.operand_stacks.borrow_mut().remove(index);
            self.pcs.borrow_mut().remove(index);
            self.frames.borrow_mut().remove(index)
        }
    }

    pub fn push_operand_value(&self, val: Value){
        if self.operand_stacks.borrow().last().unwrap().len() == self.operand_stacks.borrow().last().unwrap().capacity(){
            panic!("Method Stack overflown");
        }
        self.operand_stacks.borrow_mut().last_mut().unwrap().push(val);
    }

    pub fn pop_operand_value(&self) -> Option<Value>{
        self.operand_stacks.borrow_mut().last_mut().unwrap().pop() //TODO make it VMResult and add error type
    }

    pub fn store_local(&self, val: Value, index: usize){
        self.locals_stack.borrow_mut().last_mut().unwrap()[index] = val;
    }

    pub fn load_local(&self, index: usize) -> Option<Value>{
        self.locals_stack.borrow().last().unwrap().get(index).cloned() //TODO same as above
    }

    pub fn set_pc(&self, val: u16){
        *self.pcs.borrow_mut().last_mut().unwrap() = ProgramCounter(val);
    }

    pub fn get_pc(&self) -> ProgramCounter{
        *self.pcs.borrow().last().unwrap()
    }

    pub fn get_class_and_method_id_cloned(&self) -> ClassAndMethodId {
        self.frames.borrow().last().unwrap().class_and_method.clone()
    }

    pub fn len(&self) -> usize{
        self.frames.borrow().len()
    }

    pub fn is_empty(&self) -> bool{
        self.frames.borrow().is_empty()
    }

    pub fn print_call_stack(&self, vm: &VM) {
        for (index, call_frame_info) in self.frames.borrow().iter().enumerate(){
            //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
            warn!("[{}]: {:?}", index, self.format_frame(index, vm, &call_frame_info.class_and_method));
        }
    }

    fn format_frame(&self, index: usize, vm: &VM, cam: &ClassAndMethodId) -> String{
        let mut line_number = -1;
        let mut instruction = None;
        let pc = self.pcs.borrow()[index].0;
        let cam = ClassAndMethod::try_resolve(vm, cam).unwrap();
        if let Some(code) = &cam.method.attributes.code{
            instruction = cam.method.code_blocks.as_ref().map(|blocks| blocks.get(&pc).unwrap());
            // TODO check all tables not only the first
            if let Some(line_number_table) = &code.attributes.line_number_tables.get(0){
                for entry in line_number_table.line_number_table.iter().rev(){
                    if entry.start_pc < pc || (pc == 0 && entry.start_pc == 0) {
                        line_number = entry.line_number as i32;
                        break; 
                    }
                }
            }
        };
        format!("Method: {}:{} at {:?} ({:?})", cam.format(), line_number, pc, instruction)
    }
}