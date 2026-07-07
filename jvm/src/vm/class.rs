use crate::access_flags::{ClassFlag, MethodFlag};
use crate::class_file::attributes::ClassFileAttributes;
use crate::class_file::constant_pool::{BytecodeBehavior, ConstantPool, ConstantPoolEntry};
use crate::class_file::field_info::{native_escape, native_escaped_descriptor};
use crate::class_file::fields::field_type::FieldType;
use crate::class_file::fields::FieldInfo;
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::class_file::methods::{MethodInfo, ITABLE_INDEX_MAX, NONVIRTUAL_VTABLE_INDEX, PENDING_ITABLE_INDEX};
use crate::error::ClassParseError;
use crate::vm::result::VMResult;
use crate::vm::value::Value;
use crate::vm::VM;
use crate::vm::{ProgramCounter, VmError};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::RwLock;

#[derive()]
pub struct Class<'a>{
    pub id: ClassId,
    pub name: String,
    pub constants: RwLock<ConstantPool<'a>>,
    pub flags: u16,
    pub superclass: Option<ClassRef<'a>>,
    pub this_index: u16,
    pub interfaces: Vec<ClassRef<'a>>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub transitive_field_count: usize,
    pub first_field_index: usize,
    pub array_info: Option<ArrayInfo>,
    pub attributes: ClassFileAttributes,
}

impl<'a> Class<'a>{
    pub fn find_method(&self, method_name: &str, descriptor: &str) -> Option<&MethodInfo>{
        self.methods.iter().find(|m|  m.name == method_name && (m.descriptor.matches(descriptor) || self.has_method_polymorphic_signature(m)))
    }

    //https://docs.oracle.com/javase/specs/jvms/se8/html/jvms-2.html#jvms-2.9
    pub fn has_method_polymorphic_signature(&self, info: &MethodInfo) -> bool {
        info.flags & MethodFlag::Native as u16 > 0 && info.flags & MethodFlag::VarArgs as u16 > 0 &&
            info.descriptor.matches("([Ljava/lang/Object;)Ljava/lang/Object;") &&
            self.name == "java/lang/invoke/MethodHandle"
    }

    pub fn resolve_method_virtual(&'a self, method_name: &str, descriptor: &str) -> Option<ClassAndMethod<'a>> {
        let mut current_class = self;
        if self.is_interface(){
            loop {
                if let Some(method) = current_class.find_method(method_name, descriptor){
                    return Some(ClassAndMethod{class: current_class, method});
                }
                current_class = current_class.interfaces.first()?;
            }
        } else {
            loop {
                if let Some(method) = current_class.find_method(method_name, descriptor){
                    return Some(ClassAndMethod{class: current_class, method});
                }
                current_class = current_class.superclass?;
            }
        }
    }

    pub fn resolve_interface_method_virtual(&'a self, method_name: &str, descriptor: &str) -> Option<ClassAndMethod<'a>> {
        let mut current_class = self;
        loop {
            if let Some(method) = current_class.find_method(method_name, descriptor){
                return Some(ClassAndMethod{class: current_class, method});
            }
            let super_class = current_class.superclass?;
            if super_class.superclass.is_some(){
                current_class = super_class
            } else {
                current_class = current_class.interfaces.first()?;
            }
        }
    }

    pub fn find_method_index(&self, method_name: &str, descriptor: &str) -> Option<usize>{
        self.methods.iter().enumerate().find(|(_, m)| m.name == method_name && m.descriptor.matches(descriptor)).map(|(i, _)| i)
    }

    pub fn get_method_in_slot(&self, slot: usize) -> Option<&MethodInfo> {
        self.methods.iter().find(|m| m.slot == slot)
    }

    pub fn find_field(&self, field_name: &str) -> Option<(usize, &FieldInfo)>{
        if let Some((index, info, _)) = self.find_field_static(field_name){
            Some((index, info))
        } else {
            None
        }
    }

    //FIXME include field type in search
    pub fn find_field_static(&self, field_name: &str) -> Option<(usize, &FieldInfo, ClassId)>{
        self.fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == field_name)
            .map(|(index, field)| (index + self.first_field_index, field, self.id))
            .or_else(|| {
                if let Some(superclass) = &self.superclass{
                    superclass.find_field_static(field_name)
                } else {
                    None
                }
            })
    }

    pub fn is_interface(&self) -> bool {
        self.flags & ClassFlag::Interface as u16 > 0
    }
    pub fn is_final(&self) -> bool {
        self.flags & ClassFlag::Final as u16 > 0
    }

    pub fn is_array(&self) -> bool {
        self.array_info.is_some()
    }

    pub fn get_constant_as_value(&'a self, vm: &VM<'a>, index: u16) -> Value{
        let optional_constant = self.get_or_resolve_constant(&vm, index);
        if let Some(constant) = optional_constant{
            match constant {
                ConstantPoolEntry::Integer(value) => Value::Integer(value),
                ConstantPoolEntry::Long(value) => Value::Long(value),
                ConstantPoolEntry::Float(value) => Value::Float(value),
                ConstantPoolEntry::Double(value) => Value::Double(value),
                ConstantPoolEntry::String(_index) => vm.null(), //FIXME resolve string and allocate
                _ => {panic!("Constant of type {constant:?} not supported")}
            }
        } else {
            Value::Uninitialized
        }
    }

    pub fn get_fields(&'a self, vm: &VM<'a>) -> Vec<Value>{
        let local_values = (self.first_field_index..self.transitive_field_count)
            .map(|index| {
                let field = self.field_at_index(index).unwrap();
                if let Some(constant_value) = field.attributes.constant_value.clone(){
                    self.get_constant_as_value(&vm, constant_value.constantvalue_index)
                } else {
                    field.field_type.get_default_value(vm.null())
                }
            });
        let mut superclass_values = match self.superclass {
            Some(super_class) => super_class.get_fields(&vm),
            None => Vec::new()
        };

        superclass_values.extend(local_values);
        superclass_values
    }

    pub fn get_methods(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| !public_only || m.flags & MethodFlag::Public as u16 > 0)
            .collect()
    }

    pub fn get_constructors(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| m.name == "<init>")
            .filter(|m| !public_only || m.flags & MethodFlag::Public as u16 > 0)
            .collect()
    }

    pub fn field_at_index(&self, index: usize) -> Option<&FieldInfo>{
        if index < self.first_field_index{
            self.superclass.and_then(|superclass| superclass.field_at_index(index))
        } else {
            self.fields.get(index - self.first_field_index)
        }
    }


    pub fn init_vtable(&mut self) {
        for i in 0..self.methods.len(){
            let needs_vtable_entry = self.needs_vtable_entry(i);
            if needs_vtable_entry {
                let method = self.methods.get_mut(i).unwrap();
                method.vtable_index = method.slot as isize;
            }
        }
    }

    fn needs_vtable_entry(&mut self, index: usize) -> bool {
        let is_final = self.is_final();
        let is_interface = self.is_interface();
        let super_class = self.superclass.clone();

        let mut allocate_new: bool = true;

        let mut target_method = self.methods.get_mut(index).unwrap();

        // TODO account for default methods
        target_method.vtable_index = NONVIRTUAL_VTABLE_INDEX;

        if target_method.is_static() || target_method.name == "<init>"{
            return false;
        }

        if target_method.is_final() || is_final{
            allocate_new = false;
        } else if is_interface {
            allocate_new = false;
            if !target_method.has_itable_index() {
                target_method.vtable_index = PENDING_ITABLE_INDEX;
            }
        }

        if !super_class.is_some() {
            return allocate_new;
        }

        if target_method.is_private() {
            return allocate_new;
        }

        // https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/oops/klassVtable.cpp#L341
        let super_class = super_class.unwrap();
        // FIXME only works one layer deep
        for super_method in super_class.methods.iter() {
            if target_method == super_method {
                if !super_method.is_private() && true /* is_override */ {
                    if !target_method.is_package_private() {
                        allocate_new = false;
                    }

                    target_method.vtable_index = target_method.slot as isize;
                }
            }
        }

        allocate_new
    }

    pub fn init_itable(&mut self) {

        // assign_itable_indices_for_interface
        if self.is_interface() {
            for target_method in self.methods.iter_mut(){
                // interface_method_needs_itable_index
                if !target_method.is_static() && !target_method.is_initializer() {
                    if !target_method.has_vtable_index() {
                        assert_eq!(target_method.vtable_index, PENDING_ITABLE_INDEX);
                        target_method.vtable_index = ITABLE_INDEX_MAX - target_method.slot as isize;
                    }
                }
            }
            return;
        }
        let interfaces = self.interfaces.clone();
        for interface in interfaces.iter() {
            for interface_method in interface.methods.iter() {
                if interface_method.has_itable_index() {
                    //let cam = self.resolve_interface_method_virtual(interface_method.name.as_str(), interface_method.descriptor.as_str()).unwrap();
                    // FIXME only one layer deep
                    for target_method in self.methods.iter_mut(){
                        if target_method == interface_method {
                            target_method.vtable_index = ITABLE_INDEX_MAX - target_method.slot as isize;
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Debug for Class<'a>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Class")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("flags", &self.flags)
            .field("array_info", &self.array_info)
            .field("attributes", &self.attributes)
            .finish()
    }
}

impl PartialEq for Class<'_>{
    fn eq(&self, other: &Self) -> bool{
        self.id == other.id
    }
}

impl Eq for Class<'_>{}

pub type ClassRef<'a> = &'a Class<'a>;

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct ClassId(pub u32);

#[derive(Debug)]
pub struct ArrayInfo{
    pub(crate) dims: usize,
    pub(crate) component_type: FieldType
}

impl <'a> Class<'a>{
    pub fn get_or_resolve_constant(&self, vm: &VM<'a>, index: u16) -> Option<ConstantPoolEntry<'a>>{
        let fast = self.constants.read().ok()?.get(index as usize - 1)?.clone();
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
                let (name, typ) = resolve_name_and_type(self.constants.read().ok()?.deref(), name_index, type_index)?;
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::NameAndType(name, typ);
            }
            ConstantPoolEntry::RawString(utf8_index) => {
                let string = resolve_utf(self.constants.read().ok()?.deref(), utf8_index)?;
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::String(string);
            }
            ConstantPoolEntry::RawClass(name_index) => {
                let name = resolve_utf(self.constants.read().ok()?.deref(), name_index)?;
                let class = vm.get_or_resolve_class(name.as_str()).ok()?;
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::Class(class);
            }
            ConstantPoolEntry::RawFieldRef(class_index, name_and_type_index) => {
                let caf = resolve_class_and_field(&vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index)?;
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::FieldRef(caf);
            }
            ConstantPoolEntry::RawMethodRef(class_index, name_and_type_index) => {
                //let cam = resolve_class_and_method(&vm, &self.constants.borrow(), class_index, name_and_type_index, false)?;
                let (clazz, method_name, method_descriptor) = resolve_class_and_name_and_type(vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index)?;
                let cam = clazz.resolve_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap();
                if cam.class.has_method_polymorphic_signature(cam.method) {
                    self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::MethodRefSigPoly(cam, MethodDescriptor::new(method_descriptor));
                } else {
                    self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::MethodRef(cam);
                }
            }
            ConstantPoolEntry::RawInterfaceMethodRef(class_index, name_and_type_index) => {
                let cam = resolve_class_and_method(&vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index, true)?;
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::InterfaceMethodRef(cam);
            }
            ConstantPoolEntry::RawMethodHandle(kind, ref_index) => {
                let kind = BytecodeBehavior::from_repr(kind)?;
                match kind {
                    BytecodeBehavior::REFGetField |
                    BytecodeBehavior::REFGetStatic |
                    BytecodeBehavior::REFPutField |
                    BytecodeBehavior::REFPutStatic => {
                        let reference = self.constants.read().ok()?.deref()[ref_index as usize - 1].clone();
                        let caf = match reference {
                            ConstantPoolEntry::RawFieldRef(class_index, name_and_type_index) => resolve_class_and_field(&vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index)?,
                            ConstantPoolEntry::FieldRef(caf) => caf,
                            _ => return None,
                        };
                        self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::MethodHandleField(kind, caf);
                    }
                    _ => {
                        let reference = self.constants.read().ok()?.deref()[ref_index as usize - 1].clone();
                        let cam = match reference {
                            ConstantPoolEntry::RawMethodRef(class_index, name_and_type_index) => resolve_class_and_method(&vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index, false)?,
                            ConstantPoolEntry::RawInterfaceMethodRef(class_index, name_and_type_index) => resolve_class_and_method(&vm, self.constants.read().ok()?.deref(), class_index, name_and_type_index, true)?,
                            ConstantPoolEntry::MethodRef(cam) | ConstantPoolEntry::InterfaceMethodRef(cam) => cam,
                            _ => return None,
                        };
                        self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::MethodHandleMethod(kind, cam);
                    }
                }
            }
            ConstantPoolEntry::RawMethodType(descriptor_index) => {
                let descriptor = resolve_utf(self.constants.read().ok()?.deref(), descriptor_index).map(MethodDescriptor::new).map(ConstantPoolEntry::MethodType)?;
                self.constants.write().ok()?[index as usize - 1] = descriptor;
            }
            ConstantPoolEntry::RawInvokeDynamic(bootstrap_method_index, name_and_type_index) => {
                let bm_attribute = self.attributes.bootstrap_methods.clone()?;
                let bm = bm_attribute.bootstrap_methods.get(bootstrap_method_index as usize)?;
                let name_and_type = self.constants.read().ok()?.deref()[name_and_type_index as usize - 1].clone();
                let (handle_name, handle_type) = match name_and_type{
                    ConstantPoolEntry::RawNameAndType(name_index, type_index) => resolve_name_and_type(self.constants.read().ok()?.deref(), name_index, type_index)?,
                    ConstantPoolEntry::NameAndType(name, typ) => (name, typ),
                    _ => return None,
                };
                self.constants.write().ok()?[index as usize - 1] = ConstantPoolEntry::InvokeDynamic(bm.clone(), handle_name, handle_type);
            }
        }
        self.constants.read().ok()?.get(index as usize - 1).cloned()
    }
    
    pub fn get_constant(&self, index: u16) -> Option<ConstantPoolEntry<'a>> {
        self.constants.read().ok()?.get(index as usize - 1).cloned()
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

fn resolve_class_and_name_and_type<'a>(vm: &VM<'a>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16) -> Option<(ClassRef<'a>, String, String)>{
    let class = match constant_pool.get(class_index as usize - 1){
        Some(ConstantPoolEntry::RawClass(name_index)) => resolve_utf(constant_pool, *name_index).map(|class_name| vm.get_or_resolve_class(&class_name).ok()).flatten()?,
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

fn resolve_class_and_field<'a>(vm: &VM<'a>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16) -> Option<ClassAndField<'a>>{
    let (clazz, field_name, field_type) = resolve_class_and_name_and_type(vm, constant_pool, class_index, name_and_type_index)?;
    Some(ClassAndField{
            class: clazz,
            field: clazz.find_field(&field_name).map(|(_, info)| info).unwrap()
        }
    )
}

fn resolve_class_and_method<'a>(vm: &VM<'a>, constant_pool: &ConstantPool<'a>, class_index: u16, name_and_type_index: u16, is_interface_method: bool) -> Option<ClassAndMethod<'a>>{
    let (clazz, method_name, method_descriptor) = resolve_class_and_name_and_type(vm, constant_pool, class_index, name_and_type_index)?;
    Some(if is_interface_method{
        clazz.resolve_interface_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
    } else {
        clazz.resolve_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
    })
}

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

    pub fn get_constant_method_ref_fast(&self, vm: &VM<'a>, index: u16) -> Option<ClassAndMethod<'a>>{
        match self.class.get_or_resolve_constant(vm, index){
            Some(ConstantPoolEntry::MethodRef(cam)) |
            Some(ConstantPoolEntry::InterfaceMethodRef(cam)) => Some(cam),
            _ => None
        }
    }

    pub fn get_constant_field_ref(&self, vm: &VM<'a>, index: u16) -> Option<ClassAndField<'a>>{
        match self.class.get_or_resolve_constant(vm, index){
            Some(ConstantPoolEntry::FieldRef(caf)) => Some(caf),
            _ => None
        }
    }

    pub fn get_constant_class_ref(&self, vm: &VM<'a>, index: u16) -> Option<ClassRef<'a>>{
        match self.class.get_or_resolve_constant(vm, index) {
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

    pub fn resolve_exception_handler(&self, vm: &VM<'a>, current_pc: &ProgramCounter, thrown_class_name: &str) -> Option<u16> {
        if let Some(code) = &self.method.attributes.code {
            for handler in code.exception_table.iter() {
                let can_handle = match handler.catch_type {
                    0 => true,
                    index => {
                        let class = self.get_constant_class_ref(vm, index)?;
                        vm.unchecked_check_if_subclass_of(class.name.as_str(), thrown_class_name).ok()?
                    }
                };
                if can_handle{
                    //FIXME check if end_pc is inclusive or exclusive
                    if handler.start_pc <= current_pc.0 && current_pc.0 <= handler.end_pc{
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
        let method = clazz.get_method_in_slot(camid.method_slot).ok_or(VmError::ValidationError("Method not found".to_owned()))?;
        Ok(Self { class: clazz, method })
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