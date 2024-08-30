use std::collections::HashMap;
use thiserror::Error;

use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;
use crate::access_flags::MethodFlag;
use crate::attribute::ProgramCounter;
use crate::error::ClassParseError;
use crate::vm::call_frame::CallFrame;
use crate::vm::class::{ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::ResolvedClass;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::value::{ObjectRef, ObjectValue};

pub mod class_path;
pub mod class_path_entry;
pub mod class_manager;
mod java_error;
pub mod value;
mod call_frame;
mod class;
mod gc;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: Vec<CallFrame<'a>>,
    pub object_allocator: ObjectAllocator<'a>,
    pub static_class_objects: HashMap<ClassId, ObjectRef<'a>>,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let class_manager = ClassManager::new(class_path);
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            call_stack: Vec::new(),
            static_class_objects: HashMap::new(),
        }
    }

    pub fn invoke(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
        if !class_and_method.method.is_native(){
            let mut callframe = self.push_call_frame(class_and_method, object, args);
            callframe.execute(self)
        } else {
            println!("Native Method {}.{}({:?}) wont get executed", &class_and_method.class.name, &class_and_method.method.name, &class_and_method.method.descriptor);
            Ok(None)
        }
    }

    pub fn get_or_resolve_class(&mut self, class_name: &str) -> Result<ClassRef<'a>, VmError>{
        let resolved = self.class_manager.get_or_resolve_class(class_name)?;
        if let ResolvedClass::NewClass(to_init) = &resolved{
            for class in to_init.to_initialize.iter(){
                self.init_class(class)?;
            }
        }
        Ok(resolved.get_class())
    }

    fn init_class(&mut self, class: ClassRef<'a>) -> Result<(), VmError>{
        let static_object = self.new_object_from_class(class);
        self.static_class_objects.insert(class.id, static_object);
        if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
            let class_and_method = ClassAndMethod{
                class,
                method: clinit_method,
            };
            self.invoke(class_and_method, Some(static_object), Vec::new())?;
        }
        Ok(())
    }

    pub fn resolve_class_method(&mut self, class_name: &str, method_name: &str, descriptor: &str) -> Result<ClassAndMethod<'a>, VmError>{
        self.get_or_resolve_class(class_name)
            .and_then(|class| {
                class
                    .find_method(method_name, descriptor)
                    .map(|method| ClassAndMethod{ class, method})
                    .ok_or(VmError::JavaException(JavaError::MethodNotFoundException(method_name.to_string())))
            })
    }

    pub fn new_object(&mut self, class_name: &str) -> Result<ObjectRef<'a>, VmError>{
        let class = self.get_or_resolve_class(class_name)?;
        Ok(self.new_object_from_class(class))
    }

    pub fn new_object_from_class(&self, class: ClassRef<'a>) -> ObjectRef<'a>{
        self.object_allocator.allocate(class)
    }

    pub fn get_static_class_object(&self, id: ClassId) -> Option<ObjectRef<'a>>{
        self.static_class_objects.get(&id).cloned()
    }

    fn push_call_frame(&self, class_and_method: ClassAndMethod<'a>, object: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> CallFrame<'a>{
        let mut locals = Vec::with_capacity(class_and_method.get_max_locals());
        if let Some(obj) = object{
            locals.push(Value::Object(obj));
        }
        assert_eq!(args.len(), class_and_method.method.get_args_count(), "Args has not the correct length (was {}, expected {})", args.len(), class_and_method.method.get_args_count());
        locals.extend_from_slice(args.as_slice());
        CallFrame{
            class_and_method,
            locals,
            pc: ProgramCounter(0),
            stack: Vec::new()
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum VmError{
    #[error("")]
    JavaException(#[from] JavaError),
    #[error("")]
    ParseError(#[from] ClassParseError),
    #[error("Methodcall to {0} failed")]
    MethodCallError(String),
}