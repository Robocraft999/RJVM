use crate::access_flags::{class_flags, method_flags};
use crate::class_file::attributes::ClassFileAttributes;
use crate::class_file::constant_pool::{ConstantPool, ConstantPoolEntry};
use crate::class_file::fields::FieldInfo;
use crate::class_file::fields::field_type::FieldType;
use crate::class_file::methods::{ITABLE_INDEX_MAX, MethodInfo, NONVIRTUAL_VTABLE_INDEX, PENDING_ITABLE_INDEX};
use crate::vm::Context;
use crate::vm::class::method_tables::{ITable, VTable};
use crate::vm::value::{RefId, Value};
use class_and_member::ClassAndMethod;
use parking_lot::RwLock;
use std::collections::{HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

pub mod class_and_member;
pub mod constants;
pub mod method_tables;

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
    pub default_methods: Vec<usize>,
    pub vtable: VTable,
    pub itable: ITable,
    pub transitive_field_count: usize,
    pub transitive_method_count: usize,
    pub first_field_index: usize,
    pub first_method_index: usize,
    pub class_loader: Option<RefId>,
    pub array_info: Option<ArrayInfo>,
    pub attributes: ClassFileAttributes,
}

impl<'a> Class<'a>{
    pub fn find_method(&self, method_name: &str, descriptor: &str) -> Option<&MethodInfo>{
        self.methods.iter().find(|m|  m.name == method_name && (m.descriptor.matches(descriptor) || m.is_sig_poly))
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

    pub fn lookup_instance_method(&self, method_name: &str, descriptor: &str) -> Option<MethodInfo> {
        let mut current_class = self;
        let mut target = current_class.find_method(method_name, descriptor);
        if target.is_some(){
            return target.cloned();
        }

        while let Some(super_class) = current_class.superclass {
            current_class = super_class;
            target = current_class.find_method(method_name, descriptor);
            if target.is_some(){
                return target.cloned();
            }
        }

        if self.is_array() {
            return None;
        }

        // search default methods
        // FIXME just take the first but verify that
        let mut candidates = Vec::new();

        let transitive_interfaces = self.transitive_interfaces();
        for interface_clazz in transitive_interfaces {
            for i in &interface_clazz.default_methods {
                let default_method = &interface_clazz.methods[*i];
                if default_method.name == method_name && default_method.descriptor.as_str() == descriptor {
                    candidates.push(default_method.clone());
                }
            }
        }

        if candidates.len() > 0 {
            return Some(candidates.into_iter().next().unwrap())
        }

        None
    }

    pub fn find_method_slot(&self, method_name: &str, descriptor: &str) -> Option<usize> {
        self.methods
            .iter()
            .find(|m| m.name == method_name && m.descriptor.matches(descriptor))
            .map(|(i)| i.slot)
            .or_else(|| {
                if let Some(superclass) = &self.superclass {
                    superclass.find_method_slot(method_name, descriptor)
                } else {
                    None
                }
            })
    }

    pub fn get_method_in_slot(&'a self, slot: usize) -> Option<ClassAndMethod<'a>> {
        self.methods
            .iter()
            .find(|m| m.slot == slot)
            .map(|m| ClassAndMethod{ class: self, method: m })
            .or_else(|| {
                if let Some(superclass) = &self.superclass {
                    superclass.get_method_in_slot(slot)
                } else {
                    None
                }
            })
    }

    pub fn find_field_slot(&self, field_name: &str) -> Option<usize> {
        self.fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| f.slot)
            .or_else(|| {
                if let Some(superclass) = &self.superclass{
                    superclass.find_field_slot(field_name)
                } else {
                    None
                }
            })
    }

    pub fn get_field_in_slot(&self, slot: usize) -> Option<&FieldInfo> {
        self.fields
            .iter()
            .find(|f| f.slot == slot)
            .or_else(|| {
                if let Some(superclass) = &self.superclass{
                    superclass.get_field_in_slot(slot)
                } else {
                    None
                }
            })
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
                for clazz in self.interfaces.iter().chain(&self.superclass) {
                    if let Some(res) = clazz.find_field_static(field_name) {
                        return Some(res);
                    }
                }
                None
            })
    }

    pub fn is_interface(&self) -> bool {
        !self.is_array() && self.flags & class_flags::INTERFACE > 0
    }
    pub fn is_final(&self) -> bool {
        self.flags & class_flags::FINAL > 0
    }

    pub fn is_array(&self) -> bool {
        self.array_info.is_some()
    }

    pub fn get_constant_as_value(&'a self, ctx: &Context<'a, '_>, index: u16) -> Value{
        let optional_constant = self.get_or_resolve_constant(&ctx, index);
        if let Some(constant) = optional_constant{
            match constant {
                ConstantPoolEntry::Integer(value) => Value::Integer(value),
                ConstantPoolEntry::Long(value) => Value::Long(value),
                ConstantPoolEntry::Float(value) => Value::Float(value),
                ConstantPoolEntry::Double(value) => Value::Double(value),
                ConstantPoolEntry::String(_index) => ctx.vm.null(), //FIXME resolve string and allocate
                _ => {panic!("Constant of type {constant:?} not supported")}
            }
        } else {
            Value::Uninitialized
        }
    }

    pub fn get_fields(&'a self, ctx: &Context<'a, '_>) -> Vec<Value>{
        let local_values = (self.first_field_index..self.transitive_field_count)
            .map(|index| {
                let field = self.field_at_index(index).unwrap();
                if let Some(constant_value) = field.attributes.constant_value.clone(){
                    self.get_constant_as_value(&ctx, constant_value.constantvalue_index)
                } else {
                    field.field_type.get_default_value(ctx.vm.null())
                }
            });
        let mut superclass_values = match self.superclass {
            Some(super_class) => super_class.get_fields(&ctx),
            None => Vec::new()
        };

        superclass_values.extend(local_values);
        superclass_values
    }

    pub fn get_methods(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| !public_only || m.flags & method_flags::PUBLIC> 0)
            .collect()
    }

    pub fn get_constructors(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| m.name == "<init>")
            .filter(|m| !public_only || m.flags & method_flags::PUBLIC > 0)
            .collect()
    }

    pub fn field_at_index(&self, index: usize) -> Option<&FieldInfo>{
        if index < self.first_field_index{
            self.superclass.and_then(|superclass| superclass.field_at_index(index))
        } else {
            self.fields.get(index - self.first_field_index)
        }
    }

    // FIXME OMFG THIS IS TOO EXPENSIVE!!! CACHE THIS YOU IDIOT
    pub fn transitive_interfaces(&self) -> Vec<ClassRef<'a>> {
        let mut ids = HashSet::new();
        let mut res = Vec::new();
        let mut to_process = VecDeque::from(self.interfaces.clone());

        let mut current_class = self;
        while let Some(super_class) = current_class.superclass {
            to_process.extend(super_class.interfaces.iter());
            current_class = super_class;
        }

        while let Some(interface_clazz) = to_process.pop_front() {
            if !ids.contains(&interface_clazz.id) {
                ids.insert(interface_clazz.id);
                to_process.extend(interface_clazz.interfaces.iter());
                res.push(interface_clazz);
            }
        }
        res
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

        let target_method = self.methods.get_mut(index).unwrap();

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

#[derive(Debug, Clone)]
pub struct ArrayInfo{
    pub(crate) dims: usize,
    pub(crate) component_type: FieldType
}