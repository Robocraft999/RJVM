use std::collections::HashMap;
use log::info;
use regex::Regex;
use typed_arena::Arena;
use crate::attribute::ElementValue;
use crate::class_file::{parse_class_file, ClassFile};
use crate::error::ClassParseError;
use crate::field_info::{field_type_from_str, parse_field_type};
use crate::vm::class::{ArrayInfo, Class, ClassId, ClassRef};
use crate::vm::class_path::ClassPath;
use crate::vm::class_path_entry::ClassLoadingError;
use crate::vm::VmError;

#[derive(Debug, Clone)]
pub(crate) enum ResolvedClass<'a> {
    AlreadyLoaded(ClassRef<'a>),
    NewClass(ClassesToInitialize<'a>),
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
pub(crate) struct ClassesToInitialize<'a> {
    resolved_class: ClassRef<'a>,
    pub(crate) to_initialize: Vec<ClassRef<'a>>,
}


pub struct ClassManager<'a>{
    pub class_path: ClassPath,
    pub classes_by_name: HashMap<String, ClassRef<'a>>,
    pub classes_by_id: HashMap<ClassId, ClassRef<'a>>,
    pub classes: Arena<Class<'a>>,
    next_id: u32,
}

impl<'a> ClassManager<'a>{
    pub fn new (class_path: ClassPath) -> Self{
        Self{
            class_path,
            classes_by_name: HashMap::new(),
            classes_by_id: HashMap::new(),
            classes: Arena::with_capacity(100),
            next_id: 1,
        }
    }

    pub fn get_or_resolve_class(&mut self, class_name: &str) -> Result<ResolvedClass<'a>, VmError>{
        if let Some(loaded_class) = self.find_class_by_name(class_name){
            Ok(ResolvedClass::AlreadyLoaded(loaded_class))
        } else {
            self.resolve_class(class_name).map(ResolvedClass::NewClass)
        }
    }

    pub fn resolve_class(&mut self, class_name: &str) -> Result<ClassesToInitialize<'a>, VmError>{
        let (class_to_load_name, array_info) = self.try_create_array_class(class_name)?;
        let parsed_class = parse_class_file(&self.class_path, class_to_load_name.as_str())?;
        let next_id = self.next_id;
        self.next_id += 1;

        let mut resolved_classes = self.resolve_super_and_interfaces_and_annotations(&parsed_class)?;
        let mut super_class = parsed_class.super_class.map(|name| resolved_classes.get(&name).unwrap().get_class());
        let interfaces = parsed_class.interfaces.iter().map(|name| resolved_classes.get(name).unwrap().get_class()).collect();
        if let Some(extends) = parsed_class.runtime_visible_annotations.0.iter().find(|a| a.name == "Linternal/Extends;"){
            if let Some(pair) = extends.values.iter().find(|values| values.0 == "value"){
                if let ElementValue::String(string ) = &pair.1{
                    let new_super_class = self.get_or_resolve_class(string)?;
                    super_class = Some(new_super_class.get_class());
                    resolved_classes.insert(string.clone(), new_super_class);
                }
            }
        }

        let superclass_field_count = match super_class{
            Some(class) => class.transitive_field_count,
            None => 0,
        };
        let fields_count = parsed_class.fields.len();

        let class = Class {
            id: ClassId(next_id),
            name: class_name.to_string(),
            source_file: parsed_class.source_file,
            constants: parsed_class.constant_pool,
            flags: parsed_class.access_flags,
            superclass: super_class,
            interfaces,
            fields: parsed_class.fields,
            methods: parsed_class.methods,
            annotations: parsed_class.runtime_visible_annotations,
            transitive_field_count: superclass_field_count + fields_count,
            first_field_index: superclass_field_count,
            array_info
        };

        let class_ref = self.classes.alloc(class);


        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = class_ref;
            &*class_ptr
        };

        let mut classes_to_init: Vec<ClassRef> = Vec::new();
        for resolved_class in resolved_classes.values() {
            if let ResolvedClass::NewClass(new_class) = resolved_class {
                for to_initialize in new_class.to_initialize.iter() {
                    classes_to_init.push(to_initialize)
                }
            }
        }
        if class_ref.array_info.is_some() {
            if let ResolvedClass::NewClass(array_class) = self.get_or_resolve_class(class_to_load_name.as_str())? {
                for to_initialize in array_class.to_initialize.iter() {
                    classes_to_init.push(to_initialize)
                }
            }
        }

        classes_to_init.push(class_ref);

        self.classes_by_name.insert(class_name.to_string(), class_ref);
        self.classes_by_id.insert(class_ref.id, class_ref);
        Ok(ClassesToInitialize{
            resolved_class: class_ref,
            to_initialize: classes_to_init,
        })
    }

    fn resolve_super_and_interfaces_and_annotations(&mut self, class_file: &ClassFile) -> Result<HashMap<String, ResolvedClass<'a>>, VmError>{
        let mut resolved_classes = HashMap::new();
        if let Some(super_class_name) = &class_file.super_class{
            let resolved_class = self.get_or_resolve_class(super_class_name)?;
            resolved_classes.insert(super_class_name.clone(), resolved_class);
        }
        for interface_name in class_file.interfaces.iter(){
            let resolved_class = self.get_or_resolve_class(interface_name)?;
            resolved_classes.insert(interface_name.clone(), resolved_class);
        }
        for annotation in class_file.runtime_visible_annotations.0.iter(){
            let parsed_name = field_type_from_str(annotation.name.as_str()).to_class_name();
            let resolved_class = self.get_or_resolve_class(parsed_name.as_str())?;
            resolved_classes.insert(parsed_name, resolved_class);
        }
        Ok(resolved_classes)
    }

    fn try_create_array_class(&self, class_name: &str) -> Result<(String, Option<ArrayInfo>), VmError>{
        let r = Regex::new(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9]+);)").unwrap();
        if let Some(cap) = r.captures(class_name){
            if let Some(arr) = cap.name("array"){
                let dims = arr.len();
                let component_type = parse_field_type(cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), None);
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
        } else {
            Ok((class_name.to_string(), None))
        }
    }

    pub fn find_class_by_name(&self, class_name: &str) -> Option<ClassRef<'a>>{
        //self.classes.iter().find(|c| c.name == class_name)
        self.classes_by_name.get(class_name).cloned()
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.classes_by_id.get(&class_id).cloned()
    }
}