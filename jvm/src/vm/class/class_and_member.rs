use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::class_file::field_info::{native_escape, native_escaped_descriptor};
use crate::class_file::fields::field_type::FieldType;
use crate::class_file::fields::FieldInfo;
use crate::class_file::methods::MethodInfo;
use crate::vm::class::{ClassId, ClassRef};
use crate::vm::result::VMResult;
use crate::vm::{Context, ProgramCounter, VmError, VM};
use std::hash::Hash;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClassAndMethod<'a> {
    pub class: ClassRef<'a>,
    pub method: &'a MethodInfo,
}

impl Hash for ClassAndMethod<'_>{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.class.id.hash(state);
        self.method.name.hash(state);
        self.method.descriptor.hash(state);
    }
}

impl<'a> ClassAndMethod<'a>{

    pub fn get_constant_method_ref_fast(&self, ctx: &Context<'a, '_>, index: u16) -> Option<ClassAndMethod<'a>>{
        match self.class.get_or_resolve_constant(ctx, index){
            Some(ConstantPoolEntry::MethodRef(cam)) |
            Some(ConstantPoolEntry::InterfaceMethodRef(cam)) => Some(cam),
            _ => None
        }
    }

    pub fn get_constant_field_ref(&self, ctx: &Context<'a, '_>, index: u16) -> Option<ClassAndField<'a>>{
        match self.class.get_or_resolve_constant(ctx, index){
            Some(ConstantPoolEntry::FieldRef(caf)) => Some(caf),
            _ => None
        }
    }

    // FIXME Do error delegation if class load fails
    pub fn get_constant_class_ref(&self, ctx: &Context<'a, '_>, index: u16) -> Option<ClassRef<'a>>{
        match self.class.get_or_resolve_constant(ctx, index) {
            Some(ConstantPoolEntry::Class(class)) => Some(class),
            _ => None
        }
    }

    pub fn get_max_locals(&self) -> usize{
        if let Some(code) = &self.method.attributes.code{
            code.max_locals as usize
        } else {
            self.method.descriptor.args.iter().map(FieldType::get_locals_length).sum::<usize>() + if self.method.is_static() {0} else {1}
        }
    }

    pub fn get_max_stack_size(&self) -> usize{
        if let Some(code) = &self.method.attributes.code{
            code.max_stack as usize
        } else {
            0
        }
    }

    pub fn resolve_exception_handler(&self, ctx: &Context<'a, '_>, current_pc: &ProgramCounter, thrown_class_name: &str) -> Option<u16> {
        if let Some(code) = &self.method.attributes.code {
            for handler in code.exception_table.iter() {
                let can_handle = match handler.catch_type {
                    0 => true,
                    index => {
                        let class = self.get_constant_class_ref(ctx, index)?;
                        ctx.vm.unchecked_check_if_subclass_of(class.name.as_str(), thrown_class_name).ok()?
                    }
                };
                if can_handle{
                    // end_pc is usually exclusive but if invokes fail the pc is already advanced
                    // it would have to be reset to be still in range
                    if handler.start_pc <= current_pc.0 && current_pc.0 <= handler.end_pc {
                        return Some(handler.handler_pc);
                    }
                }
            }
        }
        None
    }

    pub fn format(&self) -> String{
        format!("{}.{}{}", self.class.name, self.method.name, self.method.descriptor.as_str())
    }

    pub fn native_escaped(&self) -> (String, String){
        let mut short = String::from("Java_");
        short.push_str(native_escape(self.class.name.as_str()).as_str());
        short.push('_');
        short.push_str(native_escape(self.method.name.as_str()).as_str());

        let mut long = short.clone();
        long.push_str("__");
        long.push_str(native_escaped_descriptor(&self.method.descriptor).as_str());

        (short, long)
    }
    
    pub fn try_resolve(vm: &VM<'a>, camid: &ClassAndMethodId) -> VMResult<Self> {
        let clazz = vm.find_class_by_id(camid.class_id).ok_or(VmError::ValidationError("Class not found".to_owned()))?;
        let cam = clazz.get_method_in_slot(camid.method_slot).ok_or(VmError::ValidationError("Method not found".to_owned()))?;
        Ok(cam)
    }
    
    pub fn as_ids(&self) -> ClassAndMethodId {
        ClassAndMethodId { class_id: self.class.id, method_slot: self.method.slot }
    }
}

#[derive(Debug, Clone)]
pub struct ClassAndField<'a>{
    pub class: ClassRef<'a>,
    pub field: &'a FieldInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub struct ClassAndMethodId{
    pub class_id: ClassId,
    pub method_slot: usize,
}