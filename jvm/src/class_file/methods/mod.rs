pub mod attributes;
pub mod descriptor;

use crate::access_flags::MethodFlag;
use crate::class_file::methods::attributes::{ExceptionTableEntry, MethodInfoAttributes};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::bytecode::InstructionBlock;
use crate::vm::ProgramCounter;
use crate::vm::VmError;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct MethodInfo{
    pub flags: u16,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub slot: usize,
    pub code_blocks: Option<BTreeMap<u16, InstructionBlock>>,
    pub attributes: MethodInfoAttributes,
}

impl MethodInfo{
    pub fn get_args_count(&self) -> usize{
        self.descriptor.args.len()
    }

    pub fn is_native(&self) -> bool {
        self.flags & MethodFlag::Native as u16 > 0
    }

    pub fn is_static(&self) -> bool{
        self.flags & MethodFlag::Static as u16 > 0
    }

    pub fn is_abstract(&self) -> bool {
        self.flags & MethodFlag::Abstract as u16 > 0
    }

    pub fn has_exception_handler(&self) -> bool {
        if let Some(code) = &self.attributes.code {
            code.exception_table.len() > 0
        } else {
            false
        }
    }

    pub fn get_exception_handlers(&self) -> Option<&Vec<ExceptionTableEntry>> {
        if let Some(code) = &self.attributes.code {
            Some(&code.exception_table)
        } else {
            None
        }
    }

    pub fn get_code_block_at(&self, pc: ProgramCounter) -> &InstructionBlock{
        &self.code_blocks.as_ref().unwrap().get(&pc.0).ok_or(VmError::ValidationError(format!("Code block out of bounds: {}, {:?}", pc.0, self.code_blocks))).unwrap()
    }

    pub fn next_pc(&self, pc: ProgramCounter) -> Option<u16>{
        self.code_blocks.as_ref().map(|blocks| blocks.range(pc.0+1..).next()).flatten().map(|t|*t.0)
    }

    pub fn previous_pc(&self, pc: ProgramCounter) -> u16{
        self.code_blocks.as_ref().map(|blocks| blocks.range(..pc.0).next_back()).flatten().map(|t|*t.0).unwrap_or(0)
    }
}

impl PartialEq for MethodInfo{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.descriptor == other.descriptor
    }
}

impl Eq for MethodInfo{}