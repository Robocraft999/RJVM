pub mod attributes;
pub mod descriptor;
pub mod code;

use crate::access_flags::method_flags;
use crate::class_file::methods::attributes::{ExceptionTableEntry, MethodInfoAttributes};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::ProgramCounter;
use crate::class_file::methods::code::{IrCode, LocatedIrInstruction, PC};
use crate::vm::class::ClassId;

#[derive(Debug, Clone)]
pub struct MethodInfo{
    pub flags: u16,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub slot: usize,
    pub vtable_index: isize,
    pub ir_code: Option<IrCode>,
    pub holder_id: ClassId,
    // FIXME store the entire holder instead.
    // This would require to store all methods including from superclass in a vtable to be useful
    // Although it would improve virtual resolving a lot
    pub is_holder_interface: bool,
    pub attributes: MethodInfoAttributes,
}

pub const ITABLE_INDEX_MAX: isize = -10;
pub const PENDING_ITABLE_INDEX: isize = -9;
pub const INVALID_VTABLE_INDEX: isize = -4;
pub const GARBAGE_VTABLE_INDEX: isize = -3;
pub const NONVIRTUAL_VTABLE_INDEX: isize = -2;

impl MethodInfo{
    pub fn get_args_count(&self) -> usize{
        self.descriptor.args.len()
    }

    pub fn is_native(&self) -> bool {
        self.flags & method_flags::NATIVE > 0
    }

    pub fn is_static(&self) -> bool{
        self.flags & method_flags::STATIC > 0
    }

    pub fn is_abstract(&self) -> bool {
        self.flags & method_flags::ABSTRACT > 0
    }

    pub fn is_final(&self) -> bool {
        self.flags & method_flags::FINAL > 0
    }
    pub fn is_synchronized(&self) -> bool {
        self.flags & method_flags::SYNCHRONIZED > 0
    }

    pub fn is_public(&self) -> bool { self.flags & method_flags::PUBLIC > 0 }
    pub fn is_private(&self) -> bool {
        self.flags & method_flags::PRIVATE > 0
    }
    pub fn is_protected(&self) -> bool { self.flags & method_flags::PROTECTED > 0 }
    pub fn is_package_private(&self) -> bool { !self.is_private() && !self.is_public() && !self.is_protected() }

    pub fn is_initializer(&self) -> bool {
        (self.name == "<init>" || self.name == "<clinit>") && self.descriptor.return_type.is_none()
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

    pub fn get_code_block_at(&self, pc: ProgramCounter) -> Option<&LocatedIrInstruction> {
        if let Some((ir_code)) = &self.ir_code {
            ir_code.get(pc.0)
        } else {
            None
        }
    }

    pub fn next_pc(&self, pc: ProgramCounter) -> PC {
        let Some(ir_code) = &self.ir_code else { unreachable!("there is no code") };
        ir_code.get(pc.0).unwrap().next_pc
    }

    pub fn has_vtable_index(&self) -> bool {
        self.vtable_index >= 0
    }

    pub fn vtable_index(&self) -> isize{
        self.vtable_index
    }

    pub fn has_itable_index(&self) -> bool {
        self.vtable_index <= ITABLE_INDEX_MAX
    }

    pub fn itable_index(&self) -> isize {
        // technically <= pending_itable_entry, but we don't have that
        assert!(self.vtable_index <= ITABLE_INDEX_MAX);
        ITABLE_INDEX_MAX - self.vtable_index
    }
}

impl PartialEq for MethodInfo{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.descriptor == other.descriptor
    }
}

impl Eq for MethodInfo{}