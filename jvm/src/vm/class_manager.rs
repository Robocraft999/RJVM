use crate::class_file::attributes::ClassFileAttributes;
use crate::class_file::constant_pool::{ConstantPoolEntry};
use crate::class_file::fields::attributes::FieldInfoAttributes;
use crate::class_file::fields::field_type::{extract_component_type_from_array_class, FieldType};
use crate::class_file::fields::{primitive_to_wrapper_name, FieldInfo};
use crate::class_file::methods::attributes::{CodeAttributes, MethodInfoAttributes};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::class_file::methods::{MethodInfo, GARBAGE_VTABLE_INDEX};
use crate::class_file::nom::parse_class_file;
use crate::error::ClassParseError;
use crate::vm::class::{ArrayInfo, Class, ClassId, ClassRef};
use crate::vm::class_path::ClassPath;
use crate::vm::result::VMResult;
use crate::vm::value::{RefId, Reference};
use crate::vm::{bytecode, VmError, VM};
use log::{info, warn};
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::collections::{HashMap};
use std::str::FromStr;
use std::sync::{Mutex, RwLock};
use typed_arena::Arena;
use crate::vm::application::thread;

#[derive(Debug, Clone)]
pub(crate) enum ResolvedClass<'a> {
    AlreadyLoaded(ClassRef<'a>),
    NewClass(ClassesToLoad<'a>),
}

impl<'a> ResolvedClass<'a> {
    pub fn get_class(&self) -> ClassRef<'a> {
        match self {
            ResolvedClass::AlreadyLoaded(class) => class,
            ResolvedClass::NewClass(classes_to_initialize) => classes_to_initialize.resolved_class,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClassesToLoad<'a> {
    resolved_class: ClassRef<'a>,
    pub(crate) to_load: Vec<ClassRef<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassLoadingState{
    LOADED,
    PREPARED,
    INITIALIZING,
    INITIALIZED,
}

#[derive(Debug)]
pub struct AnonClassInfo<'a> {
    pub clazz: ClassRef<'a>,
    pub host: Reference<'a>,
}

pub struct ClassManager<'a>{
    pub class_path: ClassPath,
    pub classes_by_name: RwLock<HashMap<String, ClassRef<'a>>>,
    pub classes_by_id: RwLock<HashMap<ClassId, ClassRef<'a>>>,
    pub class_loading_states: RwLock<HashMap<ClassId, ClassLoadingState>>,
    pub anonymous_classes: RwLock<HashMap<RefId, AnonClassInfo<'a>>>,
    pub classes: Mutex<Arena<Class<'a>>>,
    pub primitive_class_ids: RwLock<HashMap<String, ClassId>>,
    next_id: RwLock<u32>,
}

impl<'a> ClassManager<'a>{
    pub fn new (class_path: ClassPath) -> Self{
        Self{
            class_path,
            classes_by_name: RwLock::new(HashMap::new()),
            classes_by_id: RwLock::new(HashMap::new()),
            class_loading_states: RwLock::new(HashMap::new()),
            anonymous_classes: RwLock::new(HashMap::new()),
            classes: Mutex::new(Arena::with_capacity(100)),
            primitive_class_ids: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    pub fn get_or_resolve_class(&self, vm: &VM<'a>, class_name: &str) -> Result<ClassRef<'a>, VmError>{
        if let Some(loaded_class) = self.find_class_by_name(class_name){
            Ok(loaded_class)
        } else {
            self.resolve_class(&vm, class_name)
        }
    }

    fn resolve_class(&self, vm: &VM<'a>, class_name: &str) -> Result<ClassRef<'a>, VmError>{
        let (class_to_load_name, array_info) = self.try_create_array_class(class_name)?;
        let bytes = self.class_path.resolve(class_to_load_name.as_str())?.ok_or(ClassParseError::ClassResolveError(class_name.to_string()))?;
        self.parse_and_load_class(&vm, class_name, class_to_load_name.as_str(), array_info, bytes)
    }

    pub fn parse_and_load_class(&self, vm: &VM<'a>, class_name: &str, class_to_load_name: &str, array_info: Option<ArrayInfo>, bytes: Vec<u8>) -> VMResult<ClassRef<'a>>{
        let class = self.define_class(vm, Some(class_name), array_info, bytes)?;

        if class.array_info.is_some(){
            let _ = self.get_or_resolve_class(vm, class_to_load_name)?;
        }

        // alloc + register
        let class_ref = match self.classes.lock() {
            Ok(class_lock) => {
                let class_ref = class_lock.alloc(class);
                unsafe {
                    let class_ptr: *const Class<'a> = class_ref;
                    &*class_ptr
                }
            }
            Err(e) => return Err(VmError::from(e))
        };

        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = class_ref;
            &*class_ptr
        };

        self.classes_by_name.write()?.insert(class_name.to_string(), class_ref);
        self.classes_by_id.write()?.insert(class_ref.id, class_ref);
        self.class_loading_states.write()?.insert(class_ref.id, ClassLoadingState::LOADED);

        Ok(class_ref)
    }

    pub fn define_class(&self, vm: &VM<'a>, class_name: Option<&str>, array_info: Option<ArrayInfo>, bytes: Vec<u8>) -> VMResult<Class<'a>>{
        let parsed_class = parse_class_file(bytes.clone())?;
        let next_id = *self.next_id.read()?;
        *self.next_id.write()? += 1;

        let constants = parsed_class.constant_pool;
        let class_name = match class_name {
            Some(name) => Ok(name.to_owned()),
            None => {
                if let Some(ConstantPoolEntry::RawClass(name_index)) = constants.get(parsed_class.this_class as usize - 1){
                    if let Some(ConstantPoolEntry::Utf8(name)) = constants.get(*name_index as usize - 1){
                        Ok(name.clone())
                    } else {
                        Err(VmError::ParseError(ClassParseError::ConstantPoolError("Expected a utf entry".to_string())))
                    }
                } else {
                    Err(VmError::ParseError(ClassParseError::ConstantPoolError("Expected a raw class entry".to_string())))
                }
            }
        }?;

        if self.classes_by_name.read()?.contains_key(&class_name){
            warn!("Duplicate class name {}", class_name);
        }

        // shallow class
        let mut class = Class{
            id: ClassId(next_id),
            name: class_name,
            constants: RwLock::new(constants),
            flags: parsed_class.access_flags,
            superclass: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            transitive_field_count: 0,
            first_field_index: 0,
            attributes: ClassFileAttributes::default(),
            array_info,
        };

        // resolve super and interface classes
        class.superclass = if parsed_class.super_class > 0 {
            class.get_or_resolve_constant(&vm, parsed_class.super_class)
                .map(|e| if let ConstantPoolEntry::Class(clazz) = e {Some(clazz)} else {None})
                .flatten()
        } else {
            None
        };

        class.interfaces = parsed_class.interfaces.iter()
            .map(|i| class.get_or_resolve_constant(&vm, *i)
                .map(|e| if let ConstantPoolEntry::Class(clazz) = e {Some(clazz)} else {None}))
            .flatten()
            .try_collect::<Vec<ClassRef>>()
            .ok_or(VmError::ParseError(ClassParseError::ConstantPoolError(format!("Interface of {} could not be loaded.", class.name))))?;

        // build class attributes
        for ra in parsed_class.attributes.into_iter(){
            if let Some(ConstantPoolEntry::Utf8(name)) = class.get_or_resolve_constant(&vm, ra.attribute_name_index){
                class.attributes.set(name.as_str(), ra.info).unwrap();
            } else {
                warn!("Attribute of {} ({}) could not be loaded.", class.name, ra.attribute_name_index);
            }
        }

        // build fields and methods
        let super_class_field_count = match &class.superclass{
            Some(clazz) => clazz.transitive_field_count,
            None => 0,
        };
        class.fields = parsed_class.fields.iter()
            .enumerate()
            .map(|(i, raw_field)| (i, raw_field, class.get_or_resolve_constant(&vm, raw_field.name_index), class.get_or_resolve_constant(&vm, raw_field.descriptor_index)))
            .map(|optional| match optional {
                (i, raw_field, Some(ConstantPoolEntry::Utf8(name)), Some(ConstantPoolEntry::Utf8(descriptor))) => {
                    let mut field_attributes = FieldInfoAttributes::default();
                    for ra in raw_field.attributes.iter(){
                        if let Some(ConstantPoolEntry::Utf8(name)) = class.get_or_resolve_constant(&vm, ra.attribute_name_index){
                            field_attributes.set(name.as_str(), ra.info.clone()).unwrap();
                        }
                    }
                    let field_type = FieldType::from_str(descriptor.as_str()).ok()?;
                    Some(FieldInfo{
                        name,
                        attributes: field_attributes,
                        field_type,
                        slot: i + super_class_field_count,
                        holder_id: class.id,
                        flags: raw_field.access_flags,
                    })
                }
                _ => None,
            })
            .try_collect::<Vec<FieldInfo>>()
            .ok_or(VmError::ParseError(ClassParseError::ConstantPoolError(format!("Field of class '{}' could not be loaded.", class.name))))?;
        class.transitive_field_count = super_class_field_count + class.fields.len();
        class.first_field_index = super_class_field_count;

        class.methods = parsed_class.methods.iter()
            .enumerate()
            .map(|(i, raw_method)| (i, raw_method, class.get_or_resolve_constant(&vm, raw_method.name_index), class.get_or_resolve_constant(&vm, raw_method.descriptor_index)))
            .map(|optional| match optional {
                (i, raw_field, Some(ConstantPoolEntry::Utf8(name)), Some(ConstantPoolEntry::Utf8(descriptor))) => {
                    let mut method_attributes = MethodInfoAttributes::default();
                    for ra in raw_field.attributes.iter(){
                        if let Some(ConstantPoolEntry::Utf8(name)) = class.get_or_resolve_constant(&vm, ra.attribute_name_index){
                            method_attributes.set(name.as_str(), ra.info.clone()).unwrap();
                        }
                    }
                    if let Some(code) = &mut method_attributes.code {
                        let mut code_attributes = CodeAttributes::default();
                        for ra in code.raw_attributes.iter(){
                            if let Some(ConstantPoolEntry::Utf8(name)) = class.get_or_resolve_constant(&vm, ra.attribute_name_index){
                                code_attributes.set(name.as_str(), ra.info.clone()).unwrap();
                            }
                        }
                        code.attributes = code_attributes;
                    }
                    let descriptor = MethodDescriptor::new(descriptor);
                    let code_blocks = method_attributes.code.clone().map(|c| bytecode::get_blocks(&c.code));
                    Some(MethodInfo{
                        name,
                        descriptor,
                        slot: i+1,
                        vtable_index: GARBAGE_VTABLE_INDEX,
                        attributes: method_attributes,
                        is_holder_interface: class.is_interface(),
                        code_blocks,
                        flags: raw_field.access_flags,
                    })
                }
                _ => None,
            })
            .try_collect::<Vec<MethodInfo>>()
            .ok_or(VmError::ParseError(ClassParseError::ConstantPoolError(format!("Method of class '{}' could not be loaded.", class.name))))?;

        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = &class;
            &*class_ptr
        };

        thread().debug_helper.bytecode_helper.push_class(class_ref, bytes);

        class.init_vtable();
        class.init_itable();

        Ok(class)
    }
    
    fn get_or_create_primitive_class(&self, vm: &VM<'a>, name: &str) -> VMResult<ClassId> {
        if !self.primitive_class_ids.read()?.contains_key(name){
            let id = *self.next_id.read()?;
            *self.next_id.write()? += 1;
            self.primitive_class_ids.write()?.insert(name.to_owned(), ClassId(id));

            let wrapper = self.get_or_resolve_class(&vm, primitive_to_wrapper_name(name).as_str()).unwrap();
            self.classes_by_id.write()?.insert(ClassId(id), wrapper);
            self.classes_by_name.write()?.insert(name.to_owned(), wrapper);
        }
        Ok(self.primitive_class_ids.read()?.get(name).cloned().unwrap())
    }
    
    /// Use this carefully! The is no ClassRef for this id
    pub fn get_primitive_class(&self, vm: &VM<'a>, name: &str) -> ClassId {
        let Ok(id) = self.get_or_create_primitive_class(vm, name) else {
            unreachable!()
        };
        id
    }

    pub fn get_classes_to_initialize(&self, class: ClassRef<'a>) -> VMResult<Vec<ClassRef<'a>>> {
        let mut to_initialize = Vec::new();
        /*if let Some(super_class) = class.superclass{
            for clazz in self.get_classes_to_initialize(super_class)?{
                if !to_initialize.contains(&clazz){
                    to_initialize.push(clazz);
                }
            }
        }
        for interface in class.interfaces.iter(){
            for clazz in self.get_classes_to_initialize(interface)?{
                if !to_initialize.contains(&clazz){
                    to_initialize.push(clazz);
                }
            }
        }*/ // FIXME check if we can do that (background is, that we want to be able to initialize a class as far as possible before having to load necessary classes)
        to_initialize.push(class);
        Ok(to_initialize.into_iter().filter(|c| self.expect_class_state(c.id, ClassLoadingState::LOADED)).collect())
    }

    pub fn update_class_state(&self, clazz: ClassRef, new_state: ClassLoadingState){
        //TODO validate that the class existed
        let Ok(mut res) = self.class_loading_states.write() else {
            unreachable!("Could not acquire lock for class state unlock")
        };
        res.insert(clazz.id, new_state);
    }

    fn try_create_array_class(&self, class_name: &str) -> VMResult<(String, Option<ArrayInfo>)>{
        if let Ok((component_type, dims)) = extract_component_type_from_array_class(class_name){
            let new_class_name = if component_type.is_primitive() {
                // 私はこれがすきじゃないです
                primitive_to_wrapper_name(component_type.to_class_name().as_str())
            } else {
                component_type.to_class_name()
            };
            info!("{}", new_class_name);
            let array_info = ArrayInfo{
                dims,
                component_type,
            };
            Ok((new_class_name, Some(array_info)))
        } else {
            Ok((class_name.to_string(), None))
        }
    }

    pub fn find_class_by_name(&self, class_name: &str) -> Option<ClassRef<'a>>{
        //self.classes.iter().find(|c| c.name == class_name)
        self.classes_by_name.read().ok()?.get(class_name).cloned()
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.classes_by_id.read().ok()?.get(&class_id).cloned()
    }

    pub fn expect_class_state(&self, class_id: ClassId, state: ClassLoadingState) -> bool{
        if let Ok(res) = self.class_loading_states.read() {
            res.get(&class_id).map(|s| s == &state).unwrap_or(false)
        } else {
            unreachable!("Could not acquire lock for class state")
        }
    }
}