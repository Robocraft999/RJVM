use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use crate::access_flags::{ClassFlag, ClassFlags, MethodFlag};
use crate::attribute::{BootstrapMethods, VisibleRuntimeAnnotations};
use crate::constants::{ConstantPool, ConstantPoolEntry};
use crate::field_info::{native_escape, native_escaped_descriptor, FieldInfo, FieldType};
use crate::method_info::MethodInfo;
use crate::vm::value::{Reference, Value};

#[derive()]
pub struct Class<'a>{
    pub id: ClassId,
    pub name: String,
    pub source_file: Option<String>,
    pub constants: ConstantPool,
    pub flags: ClassFlags,
    pub superclass: Option<ClassRef<'a>>,
    pub interfaces: Vec<ClassRef<'a>>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub annotations: VisibleRuntimeAnnotations,
    pub bootstrap_methods: BootstrapMethods,
    pub transitive_field_count: usize,
    pub first_field_index: usize,
    pub array_info: Option<ArrayInfo>
}

impl<'a> Class<'a>{
    pub fn find_method(&self, method_name: &str, descriptor: &str) -> Option<&MethodInfo>{
        self.methods.iter().find(|m| m.name == method_name && m.descriptor.matches(descriptor))
    }

    pub fn find_method_index(&self, method_name: &str, descriptor: &str) -> Option<usize>{
        self.methods.iter().enumerate().find(|(_, m)| m.name == method_name && m.descriptor.matches(descriptor)).map(|(i, _)| i)
    }

    pub fn find_field(&self, field_name: &str) -> Option<(usize, &FieldInfo)>{
        if let Some((index, info, _)) = self.find_field_static(field_name){
            Some((index, info))
        } else {
            None
        }
    }

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
        self.flags.contains(&ClassFlag::Interface)
    }

    pub fn is_array(&self) -> bool {
        self.name.starts_with("[")
    }

    pub fn get_constant(&self, index: u16) -> Option<ConstantPoolEntry>{
        self.constants.0.get(index as usize - 1).cloned()
    }

    pub fn get_constant_as_value(&'a self, index: u16, null: Value<'a>) -> Value<'a>{
        let optional_constant = self.get_constant(index);
        if let Some(constant) = optional_constant{
            match constant {
                ConstantPoolEntry::Integer(value) => Value::Integer(value),
                ConstantPoolEntry::Long(value) => Value::Long(value),
                ConstantPoolEntry::Float(value) => Value::Float(value),
                ConstantPoolEntry::Double(value) => Value::Double(value),
                ConstantPoolEntry::String(_index) => null, //FIXME resolve string and allocate
                _ => {panic!("Constant of type {constant:?} not supported")}
            }
        } else {
            Value::Uninitialized
        }
    }

    pub fn get_fields(&'a self, null: Reference<'a>) -> Vec<Value<'a>>{
        let local_values = (self.first_field_index..self.transitive_field_count)
            .map(|index| {
                let field = self.field_at_index(index).unwrap();
                if let Some(constant_value) = field.constant_value.clone(){
                    self.get_constant_as_value(constant_value.constant_index, Value::Reference(null))
                } else {
                    field.field_type.get_default_value(Value::Reference(null))
                }
            });
        let mut superclass_values = match self.superclass {
            Some(super_class) => super_class.get_fields(null),
            None => Vec::new()
        };

        superclass_values.extend(local_values);
        superclass_values
    }

    pub fn get_methods(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| !public_only || m.flags.contains(&MethodFlag::Public))
            .collect()
    }

    pub fn get_constructors(&self, public_only: bool) -> Vec<&MethodInfo>{
        self.methods.iter()
            .filter(|m| m.name == "<init>")
            .filter(|m| !public_only || m.flags.contains(&MethodFlag::Public))
            .collect()
    }

    pub fn field_at_index(&self, index: usize) -> Option<&FieldInfo>{
        if index < self.first_field_index{
            self.superclass.and_then(|superclass| superclass.field_at_index(index))
        } else {
            self.fields.get(index - self.first_field_index)
        }
    }
}

impl<'a> Debug for Class<'a>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Class")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("source_file", &self.source_file)
            .field("constants", &self.constants)
            .field("flags", &self.flags)
            .field("array_info", &self.array_info)
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
    pub fn get_constant_utf8(&self, index: u16) -> Option<String> {
        if let Some(constant) = self.class.get_constant(index){
            match constant {
                ConstantPoolEntry::Utf8(string) => Some(string),
                ConstantPoolEntry::String(string_index) => self.get_constant_utf8(string_index),
                ConstantPoolEntry::Class(name_index) => self.get_constant_utf8(name_index),
                _ => None
            }
        } else {
            None
        }
    }

    pub fn get_constant_as_value(&self, index: u16, null: Value<'a>) -> Value<'a>{
        self.class.get_constant_as_value(index, null)
    }

    pub fn get_constant_method_info_descriptor(&self, index: u16) -> Option<(String, String, String)>{
        let (class_index, name_and_type_index) = if let Some(ConstantPoolEntry::Methodref(class_index, name_and_type_index)) = self.class.get_constant(index){
            (class_index, name_and_type_index)
        } else if let Some(ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index)) = self.class.get_constant(index){
            (class_index, name_and_type_index)
        } else {
            return None;
        };
        if let Some(ConstantPoolEntry::NameAndType(name_index, type_index)) = self.class.get_constant(name_and_type_index){
            let class_name = self.get_constant_utf8(class_index).unwrap();
            let method_name = self.get_constant_utf8(name_index).unwrap();
            let method_descriptor = self.get_constant_utf8(type_index).unwrap();
            return Some((class_name, method_name, method_descriptor.as_str().to_string()));
            /*if let Ok(class_and_method) = vm.resolve_class_method(class_name.as_str(), method_name.as_str(), method_descriptor.as_str()){
                return Some(class_and_method.clone())
            }*/
        }
        None
    }

    pub fn get_constant_field_info_descriptor(&self, index: u16) -> Option<(String, String, String)>{
        if let Some(ConstantPoolEntry::Fieldref(class_index, name_and_type_index)) = self.class.get_constant(index){
            if let Some(ConstantPoolEntry::NameAndType(name_index, type_index)) = self.class.get_constant(name_and_type_index){
                let class_name = self.get_constant_utf8(class_index).unwrap();
                let method_name = self.get_constant_utf8(name_index).unwrap();
                let field_descriptor = self.get_constant_utf8(type_index).unwrap();
                return Some((class_name, method_name, field_descriptor.as_str().to_string()));
                /*if let Ok(class_and_method) = vm.resolve_class_method(class_name.as_str(), method_name.as_str(), method_descriptor.as_str()){
                    return Some(class_and_method.clone())
                }*/
            }
        }
        None
    }

    pub fn get_max_locals(&self) -> usize{
        if let Some(code) = &self.method.code{
            code.max_locals as usize
        } else {
            self.method.descriptor.args.iter().map(FieldType::get_locals_length).sum::<usize>() + if self.method.is_static() {0} else {1}
        }
    }

    pub fn get_max_stack_size(&self) -> usize{
        if let Some(code) = &self.method.code{
            code.max_stack as usize
        } else {
            0
        }
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
}