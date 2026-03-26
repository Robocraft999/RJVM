use crate::attribute::{BootstrapMethod, BootstrapMethods, ElementValue};
use crate::class_file::constant_pool::{BytecodeBehavior, ConstantPool, ConstantPoolEntry, FastConstantPool, FastConstantPoolEntry};
use crate::class_file::field_info::{extract_component_type_from_array_class, primitive_to_wrapper_name, FieldType};
use crate::class_file::method_info::MethodInfo;
use crate::class_file::{parse_class_file, ClassFile};
use crate::error::ClassParseError;
use crate::vm::class::{ArrayInfo, Class, ClassAndField, ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_path::ClassPath;
use crate::vm::result::VMResult;
use crate::vm::{bytecode, class, VmError};
use log::info;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use typed_arena::Arena;

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


pub struct ClassManager<'a>{
    pub class_path: ClassPath,
    pub classes_by_name: RefCell<HashMap<String, ClassRef<'a>>>,
    pub classes_by_id: RefCell<HashMap<ClassId, ClassRef<'a>>>,
    pub class_loading_states: RefCell<HashMap<ClassId, ClassLoadingState>>,
    pub classes: Arena<Class<'a>>,
    pub primitive_class_ids: RefCell<HashMap<String, ClassId>>,
    next_id: RefCell<u32>,
}

impl<'a> ClassManager<'a>{
    pub fn new (class_path: ClassPath) -> Self{
        Self{
            class_path,
            classes_by_name: RefCell::new(HashMap::new()),
            classes_by_id: RefCell::new(HashMap::new()),
            class_loading_states: RefCell::new(HashMap::new()),
            classes: Arena::with_capacity(100),
            primitive_class_ids: RefCell::new(HashMap::new()),
            next_id: RefCell::new(1),
        }
    }

    pub fn get_or_resolve_class(&self, class_name: &str) -> Result<ResolvedClass<'a>, VmError>{
        if let Some(loaded_class) = self.find_class_by_name(class_name){
            Ok(ResolvedClass::AlreadyLoaded(loaded_class))
        } else {
            self.resolve_class(class_name).map(ResolvedClass::NewClass)
        }
    }

    fn resolve_class(&self, class_name: &str) -> Result<ClassesToLoad<'a>, VmError>{
        let (class_to_load_name, array_info) = self.try_create_array_class(class_name)?;
        let bytes = self.class_path.resolve(class_to_load_name.as_str()).map_err(|e| VmError::ParseError(ClassParseError::from(e)))?.ok_or(ClassParseError::ResolveError(class_name.to_string()))?;
        self.parse_and_load_class(class_name, class_to_load_name.as_str(), array_info, bytes)
    }

    pub fn parse_and_load_class(&self, class_name: &str, class_to_load_name: &str, array_info: Option<ArrayInfo>, bytes: Vec<u8>) -> VMResult<ClassesToLoad<'a>>{
        let parsed_class = parse_class_file(bytes, class_to_load_name)?;
        let next_id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;

        let mut resolved_classes = self.resolve_super_and_interfaces_and_annotations(&parsed_class)?;
        let mut super_class = parsed_class.super_class.map(|name| resolved_classes.get(&name).unwrap().get_class());
        let interfaces = parsed_class.interfaces.iter().map(|name| resolved_classes.get(name).unwrap().get_class()).collect();
        /*if let Some(extends) = parsed_class.runtime_visible_annotations.0.iter().find(|a| a.name == "Linternal/Extends;"){
            if let Some(pair) = extends.values.iter().find(|values| values.0 == "value"){
                if let ElementValue::String(string ) = &pair.1{
                    let new_super_class = self.get_or_resolve_class(string)?;
                    super_class = Some(new_super_class.get_class());
                    resolved_classes.insert(string.clone(), new_super_class);
                }
            }
        }*/

        let superclass_field_count = match super_class{
            Some(class) => class.transitive_field_count,
            None => 0,
        };
        let fields_count = parsed_class.fields.len();
        let methods: Vec<MethodInfo> = parsed_class.methods.into_iter().map(|mut t|{
            if let Some(code) = &t.attributes.code{
                t.code_blocks = Some(bytecode::get_blocks(&code.code))
            }
            t
        }).collect();

        let class = Class {
            id: ClassId(next_id),
            name: class_name.to_string(),
            fast_constants: RefCell::new(self.build_fast_constant_pool(&parsed_class.constant_pool, &parsed_class.attributes.bootstrap_methods)?),
            constants: parsed_class.constant_pool,
            flags: parsed_class.access_flags,
            superclass: super_class,
            interfaces,
            fields: parsed_class.fields,
            methods,
            transitive_field_count: superclass_field_count + fields_count,
            first_field_index: superclass_field_count,
            attributes: parsed_class.attributes,
            array_info,
        };

        let mut classes_to_load: Vec<ClassRef> = Vec::new();
        for resolved_class in resolved_classes.values() {
            if let ResolvedClass::NewClass(new_class) = resolved_class {
                for to_load in new_class.to_load.iter() {
                    classes_to_load.push(to_load)
                }
            }
        }
        if class.array_info.is_some() {
            if let ResolvedClass::NewClass(array_class) = self.get_or_resolve_class(class_to_load_name)? {
                for to_load in array_class.to_load.iter() {
                    classes_to_load.push(to_load)
                }
            }
        }

        let class_ref = self.classes.alloc(class);

        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = class_ref;
            &*class_ptr
        };

        classes_to_load.push(class_ref);

        self.classes_by_name.borrow_mut().insert(class_name.to_string(), class_ref);
        self.classes_by_id.borrow_mut().insert(class_ref.id, class_ref);
        self.class_loading_states.borrow_mut().insert(class_ref.id, ClassLoadingState::LOADED);
        Ok(ClassesToLoad {
            resolved_class: class_ref,
            to_load: classes_to_load,
        })
    }
    
    // Use this carefully! The is no ClassRef for this id
    pub fn get_primitive_class(&self, name: &str) -> ClassId{
        if !self.primitive_class_ids.borrow().contains_key(name){
            let id = *self.next_id.borrow();
            *self.next_id.borrow_mut() += 1;
            self.primitive_class_ids.borrow_mut().insert(name.to_owned(), ClassId(id));
            
            let wrapper = self.get_or_resolve_class(primitive_to_wrapper_name(name).as_str()).unwrap().get_class();
            self.classes_by_id.borrow_mut().insert(ClassId(id), wrapper);
            self.classes_by_name.borrow_mut().insert(name.to_owned(), wrapper);
        }
        self.primitive_class_ids.borrow().get(name).cloned().unwrap()
    }

    fn resolve_super_and_interfaces_and_annotations(&self, class_file: &ClassFile) -> VMResult<HashMap<String, ResolvedClass<'a>>>{
        let mut resolved_classes = HashMap::new();
        if let Some(super_class_name) = &class_file.super_class{
            let resolved_class = self.get_or_resolve_class(super_class_name)?;
            resolved_classes.insert(super_class_name.clone(), resolved_class);
        }
        for interface_name in class_file.interfaces.iter(){
            let resolved_class = self.get_or_resolve_class(interface_name)?;
            resolved_classes.insert(interface_name.clone(), resolved_class);
        }
        /*for annotation in class_file.runtime_visible_annotations.0.iter(){
            let parsed_name = FieldType::from_str(annotation.name.as_str())?.to_class_name();
            let resolved_class = self.get_or_resolve_class(parsed_name.as_str())?;
            resolved_classes.insert(parsed_name, resolved_class);
        }*/
        Ok(resolved_classes)
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
        self.class_loading_states.borrow_mut().insert(clazz.id, new_state);
    }

    fn try_create_array_class(&self, class_name: &str) -> VMResult<(String, Option<ArrayInfo>)>{
        if let Ok((component_type, dims)) = extract_component_type_from_array_class(class_name){
            let new_class_name = component_type.to_class_name();
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

    fn build_fast_constant_pool(&self, constant_pool: &ConstantPool, bootstrap_methods: &Option<BootstrapMethods>) -> VMResult<FastConstantPool<'a>> {

        let pool: FastConstantPool = constant_pool.0.iter().cloned().map(|old|{
            class::try_build_fast_constant_pool_entry(self, constant_pool, bootstrap_methods, old)
        }).flatten().collect();
        if pool.len() == constant_pool.0.len() {
            Ok(pool)
        } else {
            Err(VmError::ParseError(ClassParseError::ResolveError("Unexpected constant when building fast constant pool".to_string())))
        }
    }

    pub fn find_class_by_name(&self, class_name: &str) -> Option<ClassRef<'a>>{
        //self.classes.iter().find(|c| c.name == class_name)
        self.classes_by_name.borrow().get(class_name).cloned()
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.classes_by_id.borrow().get(&class_id).cloned()
    }

    pub fn expect_class_state(&self, class_id: ClassId, state: ClassLoadingState) -> bool{
        self.class_loading_states.borrow().get(&class_id).map(|s| s == &state).unwrap_or(false)
    }
}