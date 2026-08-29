use crate::class_file::constant_pool::{BytecodeBehavior, ConstantPool, ConstantPoolEntry};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::error::ClassParseError;
use crate::vm::class::class_and_member::{ClassAndField, ClassAndMethod};
use crate::vm::class::{Class, ClassRef};
use crate::vm::result::VMResult;
use crate::vm::{Context, VmError};
use std::collections::VecDeque;
use std::ops::Deref;

impl <'a> Class<'a>{
    pub fn get_or_resolve_constant(&self, ctx: &Context<'a, '_>, index: u16) -> Option<ConstantPoolEntry<'a>>{
        let fast = self.constants.read().get(index as usize - 1)?.clone();
        match fast{
            ConstantPoolEntry::Class(..) |
            ConstantPoolEntry::FieldRef(..) |
            ConstantPoolEntry::MethodRef(..) |
            ConstantPoolEntry::MethodRefSigPoly(..) |
            ConstantPoolEntry::InterfaceMethodRef(..) |
            ConstantPoolEntry::String(..) |
            ConstantPoolEntry::Integer(..) |
            ConstantPoolEntry::Float(..) |
            ConstantPoolEntry::Long(..) |
            ConstantPoolEntry::Double(..) |
            ConstantPoolEntry::Utf8(..) |
            ConstantPoolEntry::NameAndType(..) |
            ConstantPoolEntry::MethodHandleField(..) |
            ConstantPoolEntry::MethodHandleMethod(..) |
            ConstantPoolEntry::MethodType(..) |
            ConstantPoolEntry::InvokeDynamic(..) |
            ConstantPoolEntry::Dummy => {}
            ConstantPoolEntry::RawNameAndType(name_index, type_index) => {
                let (name, typ) = resolve_name_and_type(self.constants.read().deref(), name_index, type_index)?;
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::NameAndType(name, typ);
            }
            ConstantPoolEntry::RawString(utf8_index) => {
                let string = resolve_utf(self.constants.read().deref(), utf8_index)?;
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::String(string);
            }
            ConstantPoolEntry::RawClass(name_index) => {
                let name = resolve_utf(self.constants.read().deref(), name_index)?;
                let class = ctx.get_or_resolve_class(name.as_str()).ok()?;
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::Class(class);
            }
            ConstantPoolEntry::RawFieldRef(class_index, name_and_type_index) => {
                let caf = resolve_class_and_field(&ctx, self.constants.read().deref(), class_index, name_and_type_index)?;
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::FieldRef(caf);
            }
            ConstantPoolEntry::RawMethodRef(class_index, name_and_type_index) => {
                let entry = resolve_method_ref(ctx, self.constants.read().deref(), class_index, name_and_type_index).unwrap();
                self.constants.write()[index as usize - 1] = entry;
            }
            ConstantPoolEntry::RawInterfaceMethodRef(class_index, name_and_type_index) => {
                let cam = resolve_class_and_method(&ctx, self.constants.read().deref(), class_index, name_and_type_index, true)?;
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::InterfaceMethodRef(cam);
            }
            ConstantPoolEntry::RawMethodHandle(kind, ref_index) => {
                let kind = BytecodeBehavior::from_repr(kind)?;
                match kind {
                    BytecodeBehavior::REFGetField |
                    BytecodeBehavior::REFGetStatic |
                    BytecodeBehavior::REFPutField |
                    BytecodeBehavior::REFPutStatic => {
                        let reference = self.constants.read().deref()[ref_index as usize - 1].clone();
                        let caf = match reference {
                            ConstantPoolEntry::RawFieldRef(class_index, name_and_type_index) => resolve_class_and_field(&ctx, self.constants.read().deref(), class_index, name_and_type_index)?,
                            ConstantPoolEntry::FieldRef(caf) => caf,
                            _ => return None,
                        };
                        self.constants.write()[index as usize - 1] = ConstantPoolEntry::MethodHandleField(kind, caf);
                    }
                    _ => {
                        let reference = self.constants.read().deref()[ref_index as usize - 1].clone();
                        let cam = match reference {
                            ConstantPoolEntry::RawMethodRef(class_index, name_and_type_index) => resolve_class_and_method(&ctx, self.constants.read().deref(), class_index, name_and_type_index, false)?,
                            ConstantPoolEntry::RawInterfaceMethodRef(class_index, name_and_type_index) => resolve_class_and_method(&ctx, self.constants.read().deref(), class_index, name_and_type_index, true)?,
                            ConstantPoolEntry::MethodRef(cam) | ConstantPoolEntry::InterfaceMethodRef(cam) => cam,
                            _ => return None,
                        };
                        self.constants.write()[index as usize - 1] = ConstantPoolEntry::MethodHandleMethod(kind, cam);
                    }
                }
            }
            ConstantPoolEntry::RawMethodType(descriptor_index) => {
                let descriptor = resolve_utf(self.constants.read().deref(), descriptor_index).map(MethodDescriptor::new).map(ConstantPoolEntry::MethodType)?;
                self.constants.write()[index as usize - 1] = descriptor;
            }
            ConstantPoolEntry::RawInvokeDynamic(bootstrap_method_index, name_and_type_index) => {
                let bm_attribute = self.attributes.bootstrap_methods.clone()?;
                let bm = bm_attribute.bootstrap_methods.get(bootstrap_method_index as usize)?;
                let name_and_type = self.constants.read().deref()[name_and_type_index as usize - 1].clone();
                let (handle_name, handle_type) = match name_and_type{
                    ConstantPoolEntry::RawNameAndType(name_index, type_index) => resolve_name_and_type(self.constants.read().deref(), name_index, type_index)?,
                    ConstantPoolEntry::NameAndType(name, typ) => (name, typ),
                    _ => return None,
                };
                self.constants.write()[index as usize - 1] = ConstantPoolEntry::InvokeDynamic(bm.clone(), handle_name, handle_type);
            }
        }
        self.constants.read().get(index as usize - 1).cloned()
    }

    pub fn get_constant(&self, index: u16) -> Option<ConstantPoolEntry<'a>> {
        self.constants.read().get(index as usize - 1).cloned()
    }

    pub fn get_utf_constant(&self, index: u16) -> VMResult<String> {
        match self.get_constant(index) {
            Some(ConstantPoolEntry::Utf8(string)) => Ok(string),
            Some(entry) => Err(VmError::ParseError(ClassParseError::ConstantPoolError(format!("CP entry at {} is not of type UTF8 but: {:?}", index, entry)))),
            None => Err(VmError::ParseError(ClassParseError::ConstantPoolError(format!("CP entry at {} is not present", index)))),
        }
    }
}

fn resolve_utf(constant_pool: &ConstantPool, name_index: u16) -> Option<String>{
    match constant_pool.get(name_index as usize - 1){
        Some(ConstantPoolEntry::Utf8(utf)) => Some(utf.clone()),
        _ => None,
    }
}

fn resolve_name_and_type(constant_pool: &ConstantPool, name_index: u16, type_index: u16) -> Option<(String, String)>{
    match (constant_pool.get(name_index as usize - 1), constant_pool.get(type_index as usize - 1)){
        (Some(ConstantPoolEntry::Utf8(name_utf)), Some(ConstantPoolEntry::Utf8(type_utf))) => Some((name_utf.clone(), type_utf.clone())),
        _ => None,
    }
}

fn resolve_class_and_name_and_type<'a>(ctx: &Context<'a, '_>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16) -> Option<(ClassRef<'a>, String, String)>{
    let class = match constant_pool.get(class_index as usize - 1){
        Some(ConstantPoolEntry::RawClass(name_index)) => resolve_utf(constant_pool, *name_index).map(|class_name| ctx.get_or_resolve_class(&class_name).ok()).flatten()?,
        Some(ConstantPoolEntry::Class(class)) => *class,
        _ => return None,
    };
    let (name, typ) = match constant_pool.get(name_and_type_index as usize - 1){
        Some(ConstantPoolEntry::RawNameAndType(name_index, type_index)) => resolve_name_and_type(constant_pool, *name_index, *type_index)?,
        Some(ConstantPoolEntry::NameAndType(name, typ)) => (name.clone(), typ.clone()),
        _ => return None,
    };
    Some((class, name, typ))
}

fn resolve_class_and_field<'a>(ctx: &Context<'a, '_>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16) -> Option<ClassAndField<'a>>{
    let (clazz, field_name, field_type) = resolve_class_and_name_and_type(ctx, constant_pool, class_index, name_and_type_index)?;
    Some(ClassAndField{
        class: clazz,
        field: clazz.find_field(&field_name).map(|(_, info)| info).unwrap()
    }
    )
}

fn resolve_class_and_method<'a>(ctx: &Context<'a, '_>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16, is_interface_method: bool) -> Option<ClassAndMethod<'a>>{
    let (clazz, method_name, method_descriptor) = resolve_class_and_name_and_type(ctx, constant_pool, class_index, name_and_type_index)?;
    Some(if is_interface_method{
        clazz.resolve_interface_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
    } else {
        clazz.resolve_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
    })
}

fn resolve_method_ref<'a>(ctx: &Context<'a, '_>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16) -> Option<ConstantPoolEntry<'a>> {
    let (clazz, method_name, method_descriptor) = resolve_class_and_name_and_type(ctx, constant_pool, class_index, name_and_type_index)?;
    // 1.
    if clazz.is_interface() {
        // TODO IncompatibleClassChangeError
        return None;
    }
    // 2
    let mut current_class = clazz;
    loop {
        if let Some(m) = current_class.find_method(method_name.as_str(), method_descriptor.as_str()) {
            let cam = ClassAndMethod{ class: current_class, method: m };
            return if current_class.has_method_polymorphic_signature(m) {
                Some(ConstantPoolEntry::MethodRefSigPoly(cam, MethodDescriptor::new(method_descriptor)))
            } else {
                Some(ConstantPoolEntry::MethodRef(cam))
            }
        }
        match &current_class.superclass {
            Some(super_class) => current_class = super_class,
            None => break,
        }
    }
    // 3.1
    let mut interface_queue = VecDeque::from(clazz.interfaces.clone());
    while let Some(super_interface) = interface_queue.pop_front() {
        if let Some(m) = super_interface.find_method(method_name.as_str(), method_descriptor.as_str()) && !m.is_abstract() {
            return Some(ConstantPoolEntry::MethodRef(ClassAndMethod{ class: super_interface, method: m }));
        }
        interface_queue.extend(super_interface.interfaces.iter());
    }
    // 3.2
    interface_queue = VecDeque::from(clazz.interfaces.clone());
    while let Some(super_interface) = interface_queue.pop_front() {
        if let Some(m) = super_interface.find_method(method_name.as_str(), method_descriptor.as_str()) && !m.is_private() && !m.is_static() {
            return Some(ConstantPoolEntry::MethodRef(ClassAndMethod{ class: super_interface, method: m }));
        }
        interface_queue.extend(super_interface.interfaces.iter());
    }
    // 3.3
    None
}