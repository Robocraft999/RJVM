use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use crate::access_flags::{ClassFlag, ClassFlags, MethodFlag};
use crate::attribute::{BootstrapMethods, ClassFileAttributes};
use crate::constants::{BytecodeBehavior, ConstantPool, ConstantPoolEntry, FastConstantPool, FastConstantPoolEntry};
use crate::field_info::{native_escape, native_escaped_descriptor, FieldInfo, FieldType};
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::vm::class_manager::ClassManager;
use crate::vm::value::{Reference, Value};
use crate::vm::VM;

#[derive()]
pub struct Class<'a>{
    pub id: ClassId,
    pub name: String,
    pub constants: ConstantPool,
    pub fast_constants: RefCell<FastConstantPool<'a>>,
    pub flags: ClassFlags,
    pub superclass: Option<ClassRef<'a>>,
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
    fn has_method_polymorphic_signature(&self, info: &MethodInfo) -> bool {
        info.flags.contains(&MethodFlag::Native) && info.flags.contains(&MethodFlag::VarArgs) &&
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
            .field("constants", &self.constants)
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
    pub fn get_or_resolve_constant_fast(&self, vm: &VM<'a>, index: u16) -> Option<FastConstantPoolEntry<'a>>{
        let fast = self.fast_constants.borrow().get(index as usize - 1)?.clone();
        match fast{
            FastConstantPoolEntry::Class(..) |
            FastConstantPoolEntry::FieldRef(..) |
            FastConstantPoolEntry::MethodRef(..) |
            FastConstantPoolEntry::InterfaceMethodRef(..) |
            FastConstantPoolEntry::String(..) |
            FastConstantPoolEntry::Integer(..) |
            FastConstantPoolEntry::Float(..) |
            FastConstantPoolEntry::Long(..) |
            FastConstantPoolEntry::Double(..) |
            FastConstantPoolEntry::Utf8(..) |
            FastConstantPoolEntry::NameAndType(..) |
            FastConstantPoolEntry::MethodHandleField(..) |
            FastConstantPoolEntry::MethodHandleMethod(..) |
            FastConstantPoolEntry::MethodType(..) |
            FastConstantPoolEntry::InvokeDynamic(..) |
            FastConstantPoolEntry::Dummy => {}
            FastConstantPoolEntry::RawClass(class_name) => {
                let class = vm.get_or_resolve_class(class_name.as_str()).ok()?;
                self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::Class(class);
            }
            FastConstantPoolEntry::RawFieldRef(class_name, field_name, field_descriptor) => {
                let class = vm.get_or_resolve_class(class_name.as_str()).ok()?;
                let (_, field) = class.find_field(field_name.as_str())?;
                self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::FieldRef(ClassAndField{class, field});
            }
            FastConstantPoolEntry::RawMethodRef(class_name, method_name, method_descriptor) => {
                let class = vm.get_or_resolve_class(class_name.as_str()).ok()?;
                let cam = class.resolve_method_virtual(method_name.as_str(), method_descriptor.as_str())?;
                self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::MethodRef(cam);
            }
            FastConstantPoolEntry::RawInterfaceMethodRef(class_name, method_name, method_descriptor) => {
                let class = vm.get_or_resolve_class(class_name.as_str()).ok()?;
                let cam = class.resolve_interface_method_virtual(method_name.as_str(), method_descriptor.as_str())?;
                self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::InterfaceMethodRef(cam);
            }
            FastConstantPoolEntry::RawMethodHandle(kind, class_name, handle_name, handle_type) => {
                let class = vm.get_or_resolve_class(class_name.as_str()).ok()?;
                match kind {
                    BytecodeBehavior::REFGetField |
                    BytecodeBehavior::REFGetStatic |
                    BytecodeBehavior::REFPutField |
                    BytecodeBehavior::REFPutStatic => {
                        let (_, field) = class.find_field(handle_name.as_str())?;
                        self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::MethodHandleField(kind, ClassAndField{class, field});
                    }
                    _ => {
                        let cam = class.resolve_method_virtual(handle_name.as_str(), handle_type.as_str())?;
                        self.fast_constants.borrow_mut()[index as usize - 1] = FastConstantPoolEntry::MethodHandleMethod(kind, cam);
                    }
                }
            }
        }
        self.fast_constants.borrow().get(index as usize - 1).cloned()
    }
}

fn resolve_utf(constant_pool: &ConstantPool, name_index: u16) -> Option<String>{
    match constant_pool.0.get(name_index as usize - 1){
        Some(ConstantPoolEntry::Utf8(utf)) => Some(utf.clone()),
        _ => None,
    }
}

fn resolve_name_and_type(constant_pool: &ConstantPool, name_index: u16, type_index: u16) -> Option<(String, String)>{
    match (constant_pool.0.get(name_index as usize - 1), constant_pool.0.get(type_index as usize - 1)){
        (Some(ConstantPoolEntry::Utf8(name_utf)), Some(ConstantPoolEntry::Utf8(type_utf))) => Some((name_utf.clone(), type_utf.clone())),
        _ => None,
    }
}

fn resolve_class_and_field<'a>(cm: &ClassManager<'a>, constant_pool: &ConstantPool, class_index: u16, name_and_type_index: u16) -> Option<Result<ClassAndField<'a>, (String, String, String)>>{
    let class_name = match constant_pool.0.get(class_index as usize - 1){
        Some(ConstantPoolEntry::Class(name_index)) => resolve_utf(constant_pool, *name_index),
        _ => return None,
    };
    let (field_name, field_type) = match constant_pool.0.get(name_and_type_index as usize - 1){
        Some(ConstantPoolEntry::NameAndType(name_index, type_index)) => resolve_name_and_type(constant_pool, *name_index, *type_index)?,
        _ => return None,
    };
    class_name
        .map(|class_name| cm.find_class_by_name(&class_name).map_or(
            Err((class_name.clone(), field_name.clone(), field_type.clone())),
            |clazz| Ok(ClassAndField{
                class: clazz,
                field: clazz.find_field(&field_name).map(|(_, info)| info).unwrap()
            }))
        )
}

fn resolve_class_and_method<'a>(cm: &ClassManager<'a>, constant_pool: &ConstantPool, class_index: u16, name_and_type_index: u16, is_interface_method: bool) -> Option<Result<ClassAndMethod<'a>, (String, String, String)>>{
    let class_name = match constant_pool.0.get(class_index as usize - 1){
        Some(ConstantPoolEntry::Class(name_index)) => resolve_utf(constant_pool, *name_index),
        _ => return None,
    };
    let (method_name, method_descriptor) = match constant_pool.0.get(name_and_type_index as usize - 1){
        Some(ConstantPoolEntry::NameAndType(name_index, type_index)) => resolve_name_and_type(constant_pool, *name_index, *type_index)?,
        _ => return None,
    };
    class_name
        .map(|class_name| cm.find_class_by_name(&class_name).map_or(
            Err((class_name.clone(), method_name.clone(), method_descriptor.clone())),
            |clazz| Ok(if is_interface_method{
                clazz.resolve_interface_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
            } else {
                clazz.resolve_method_virtual(method_name.as_str(), method_descriptor.as_str()).unwrap()
            }))
        )
}

pub fn try_build_fast_constant_pool_entry<'a>(cm: &ClassManager<'a>, constant_pool: &ConstantPool, bootstrap_methods: &Option<BootstrapMethods>, entry: ConstantPoolEntry) -> Option<FastConstantPoolEntry<'a>>{
    match entry{
        ConstantPoolEntry::Class(name_index) => {
            let name = resolve_utf(constant_pool, name_index);
            name.map(|name| cm.find_class_by_name(name.as_str()).map_or(FastConstantPoolEntry::RawClass(name), FastConstantPoolEntry::Class))
        }
        ConstantPoolEntry::Fieldref(class_index, name_and_type_index) => {
            let f = resolve_class_and_field(cm, constant_pool, class_index, name_and_type_index);
            f.map(|res| match res {
                Ok(caf) =>
                    FastConstantPoolEntry::FieldRef(caf),
                Err((class_name, field_name, field_type)) =>
                    FastConstantPoolEntry::RawFieldRef(class_name, field_name, field_type),
            })
        }
        ConstantPoolEntry::Methodref(class_index, name_and_type_index) => {
            let m = resolve_class_and_method(cm, constant_pool, class_index, name_and_type_index, false);
            m.map(|res| match res {
                Ok(cam) =>
                    FastConstantPoolEntry::MethodRef(cam),
                Err((class_name, method_name, method_descriptor)) =>
                    FastConstantPoolEntry::RawMethodRef(class_name, method_name, method_descriptor),
            })
        }
        ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index) => {
            let m = resolve_class_and_method(cm, constant_pool, class_index, name_and_type_index, true);
            m.map(|res| match res {
                Ok(cam) =>
                    FastConstantPoolEntry::InterfaceMethodRef(cam),
                Err((class_name, method_name, method_descriptor)) =>
                    FastConstantPoolEntry::RawInterfaceMethodRef(class_name, method_name, method_descriptor),
            })
        }
        ConstantPoolEntry::Integer(value) => Some(FastConstantPoolEntry::Integer(value)),
        ConstantPoolEntry::Float(value) => Some(FastConstantPoolEntry::Float(value)),
        ConstantPoolEntry::Long(value) => Some(FastConstantPoolEntry::Long(value)),
        ConstantPoolEntry::Double(value) => Some(FastConstantPoolEntry::Double(value)),

        ConstantPoolEntry::String(value) => resolve_utf(constant_pool, value).map(FastConstantPoolEntry::String),

        ConstantPoolEntry::NameAndType(name, type_index) =>
            resolve_name_and_type(constant_pool, name, type_index).map(|(c, nat)|FastConstantPoolEntry::NameAndType(c, nat)),

        ConstantPoolEntry::Utf8(value) => Some(FastConstantPoolEntry::Utf8(value)),

        ConstantPoolEntry::MethodHandle(kind, reference_index) => {
            let kind = BytecodeBehavior::from_repr(kind)?;
            match kind{
                BytecodeBehavior::REFGetField |
                BytecodeBehavior::REFGetStatic |
                BytecodeBehavior::REFPutField |
                BytecodeBehavior::REFPutStatic => {
                    let t = match constant_pool.0.get(reference_index as usize - 1){
                        Some(ConstantPoolEntry::Fieldref(class_index, name_and_type_index)) => resolve_class_and_field(cm, constant_pool, *class_index, *name_and_type_index),
                        _ => return None,
                    };
                    t.map(|res| match res{
                        Ok(caf) =>
                            FastConstantPoolEntry::MethodHandleField(kind, caf),
                        Err((class_name, field_name, field_type)) =>
                            FastConstantPoolEntry::RawMethodHandle(kind, class_name, field_name, field_type),
                    })
                }
                BytecodeBehavior::REFInvokeVirtual |
                BytecodeBehavior::REFNewInvokeSpecial |
                BytecodeBehavior::REFInvokeStatic |
                BytecodeBehavior::REFInvokeSpecial |
                BytecodeBehavior::REFInvokeInterface => {
                    let t = match constant_pool.0.get(reference_index as usize - 1){
                        Some(ConstantPoolEntry::Methodref(class_index, name_and_type_index)) =>
                            resolve_class_and_method(cm, constant_pool, *class_index, *name_and_type_index, false),
                        Some(ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index)) =>
                            resolve_class_and_method(cm, constant_pool, *class_index, *name_and_type_index, true),
                        _ => return None,
                    };
                    t.map(|res| match res{
                        Ok(cam) =>
                            FastConstantPoolEntry::MethodHandleMethod(kind, cam),
                        Err((class_name, method_name, method_descriptor)) =>
                            FastConstantPoolEntry::RawMethodHandle(kind, class_name, method_name, method_descriptor),
                    })
                }
            }
        }
        ConstantPoolEntry::MethodType(descriptor) => resolve_utf(constant_pool, descriptor).map(MethodDescriptor::new).map(FastConstantPoolEntry::MethodType),
        ConstantPoolEntry::InvokeDynamic(bootstrap_method_index, name_and_type_index) => {
            if let Some(bootstrap_methods) = bootstrap_methods {
                bootstrap_methods.0.get(bootstrap_method_index as usize).map(|bm| {
                    let (handle_name, handle_type) = match constant_pool.0.get(name_and_type_index as usize - 1){
                        Some(ConstantPoolEntry::NameAndType(name_index, type_index)) => resolve_name_and_type(constant_pool, *name_index, *type_index)?,
                        _ => return None,
                    };
                    Some(FastConstantPoolEntry::InvokeDynamic(bm.clone(), handle_name.clone(), handle_type.clone()))
                }).flatten()
            } else {
                None
            }
        }
        ConstantPoolEntry::Dummy => Some(FastConstantPoolEntry::Dummy),
    }
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
        let (class_index, name_and_type_index) = match self.class.get_constant(index){
            Some(ConstantPoolEntry::Methodref(class_index, name_and_type_index)) |
            Some(ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index)) => {(class_index, name_and_type_index)},
            _ => return None
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

    pub fn get_constant_method_ref_fast(&self, vm: &VM<'a>, index: u16) -> Option<ClassAndMethod<'a>>{
        match self.class.get_or_resolve_constant_fast(vm, index){
            Some(FastConstantPoolEntry::MethodRef(cam)) |
            Some(FastConstantPoolEntry::InterfaceMethodRef(cam)) => Some(cam),
            _ => None
        }
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

#[derive(Debug, Clone)]
pub struct ClassAndField<'a>{
    pub class: ClassRef<'a>,
    pub field: &'a FieldInfo,
}