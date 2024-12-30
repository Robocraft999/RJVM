use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use std::str::Utf8Error;
use cesu8::{from_java_cesu8, to_java_cesu8, Cesu8DecodingError};
use callstack::CallStack;
use log::{debug, error, info, trace};
use thiserror::Error;

use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;
use crate::access_flags::MethodFlag;
use crate::attribute::ProgramCounter;
use crate::error::ClassParseError;
use crate::field_info::{FieldType, PrimitiveType};
use crate::vm::call_frame::CallFrame;
use crate::vm::class::{ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::ResolvedClass;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::java_native_method_impl::{NativeMethodRegistry, register_all_natives};
use crate::vm::r#unsafe::Unsafe;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType};

pub mod class_path;
pub mod class_path_entry;
pub mod class_manager;
mod java_error;
pub mod value;
mod call_frame;
mod callstack;
pub mod class;
mod gc;
mod java_native_method_impl;
mod r#unsafe;
mod result;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub static_class_objects: HashMap<ClassId, Reference<'a>>,
    pub string_objects: HashMap<String, Reference<'a>>,
    pub class_objects: HashMap<ClassId, Reference<'a>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub currently_open_files: HashMap<String, (Vec<u8>, usize)>,
    pub current_thread: Option<Reference<'a>>,
    pub init_call_stack: CallStack<'a>,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let class_manager = ClassManager::new(class_path);
        let mut native_method_registry = NativeMethodRegistry::new();
        register_all_natives(&mut native_method_registry);
        let unsafe_allocator = Unsafe::new();
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            unsafe_allocator,
            call_stack: CallStack::new(),
            static_class_objects: HashMap::new(),
            string_objects: HashMap::new(),
            class_objects: HashMap::new(),
            native_method_registry,
            currently_open_files: HashMap::new(),
            current_thread: None,
            init_call_stack: CallStack::new(),
        }
    }

    pub fn dump_class_file(&mut self, class_name: &str) -> VMResult<()>{
        let class = self.get_or_resolve_class(class_name)?;
        info!("Class: {:?}", class);
        Ok(())
    }

    pub fn invoke_new_frame(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
        let frame = CallStack::create_call_frame(class_and_method, object, args)?;
        self.invoke_frame(frame)
    }

    pub fn invoke_frame(&mut self, call_frame: CallFrame<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
        let class_and_method = call_frame.class_and_method.clone();
        if !class_and_method.method.is_native(){
            self.call_stack.push_call_frame(call_frame);
            self.call_stack.print_call_stack();
            let vm_ptr: *mut VM = self;
            loop {
                let partial = self.call_stack.execute_top(vm_ptr)?;
                match partial {
                    VMResultType::Ok(optional_value) => {
                        return Ok(VMResultType::Ok(optional_value));
                    }
                    VMResultType::CallPaused(new_frame) => {
                        let result = self.invoke_frame(new_frame)?;
                        if let VMResultType::Ok(returned_value) = result {
                            self.call_stack.add_to_top_stack(returned_value);
                        } else if let VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)), throwable) = result{
                            let (mut is_handled, mut handler_option) = (false, None);
                            for handler in class_and_method.method.get_exception_handlers().0 {
                                let can_handle = match handler.catch_type {
                                    Some(ref class_name) => {
                                        self.check_if_subclass_of(class_name.as_str(), thrown_class_name.as_str())?
                                    }
                                    None => true
                                };
                                if can_handle{
                                    is_handled = true;
                                    handler_option = Some(handler);
                                    break;
                                }
                            }
                            if is_handled{
                                let frame = self.call_stack.frames.last_mut().unwrap();
                                frame.pc = handler_option.unwrap().handler_pc;
                                frame.stack.push(throwable);
                                debug!("Exception thrown handled by {}", frame.class_and_method.method.name.as_str());
                            } else {
                                debug!("Exception handler not in this function");
                                return Ok(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)), throwable));
                            }
                        }
                    }
                    VMResultType::ExceptionThrown(error, throwable) => {
                        if let VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)) = &error{
                            let mut frame = self.call_stack.pop_call_frame();
                            debug!("Exception thrown by {}: {}: {}", class_and_method.format(), thrown_class_name, message);
                            if class_and_method.method.has_exception_handler() {
                                todo!()
                            } else {
                                return Ok(VMResultType::ExceptionThrown(error, throwable));
                            }
                        } else {
                            unreachable!("Could not handle {} thrown by a function", error);
                        }
                    }
                }
            }
        } else {
            let object = if class_and_method.method.is_static() {
                None
            } else {
                match call_frame.locals.get(0) {
                    Some(local) => {
                        Some(local.expect_reference()?)
                    },
                    None => None
                }
            };
            let args = call_frame.locals
                .iter()
                .cloned()
                .skip(if object.is_none() {0} else {1})
                .take_while(|value| value != &Value::Uninitialized)
                .collect::<Vec<_>>();
            let try_native = NativeMethodRegistry::invoke(self, &class_and_method, object, args);
            if let Some(native) = try_native{
                trace!("'Pushing' native method frame {:?}", call_frame);
                //self.call_stack.push_call_frame(call_frame);
                self.call_stack.print_call_stack();
                //self.handle_result(native?)
                trace!("native result is {:?}", native);
                match native? {
                    VMResultType::Ok(optional_value) => {
                        Ok(VMResultType::Ok(optional_value))
                    }
                    VMResultType::CallPaused(new_frame) => {
                        let result = self.invoke_frame(new_frame)?;
                        //should always return VMResultType::Ok(None)
                        assert!(result.is_ok());
                        /*if let VMResultType::Ok(returned_value) = result {
                            self.call_stack.add_to_top_stack(returned_value);
                        }*/
                        Ok(result)
                    }
                    VMResultType::ExceptionThrown(error, throwable) => {
                        Err(error)
                    }
                }
            } else {
                if class_and_method.method.descriptor.return_type.is_some(){
                    error!("Native Method {} wont get executed but return value is probably expected", class_and_method.format());
                    Err(VmError::MethodCallError(format!("Native {}", class_and_method.format())))
                } else {
                    info!("Native Method {} wont get executed", class_and_method.format());
                    Ok(VMResultType::Ok(None))
                    //Err(VmError::MethodCallError(format!("void Native {}", class_and_method.format())))
                }
            }
        }
    }
    
    fn invoke_frame_on_stack(&mut self, call_frame: CallFrame<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
        let class_and_method = call_frame.class_and_method.clone();
        if !class_and_method.method.is_native(){
            self.init_call_stack.push_call_frame(call_frame);
            self.init_call_stack.print_call_stack();
            let vm_ptr: *mut VM = self;
            loop {
                let partial = self.init_call_stack.execute_top(vm_ptr)?;
                match partial {
                    VMResultType::Ok(optional_value) => {
                        return Ok(VMResultType::Ok(optional_value));
                    }
                    VMResultType::CallPaused(new_frame) => {
                        let result = self.invoke_frame(new_frame)?;
                        if let VMResultType::Ok(returned_value) = result {
                            self.init_call_stack.add_to_top_stack(returned_value);
                        } else if let VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)), throwable) = result{
                            let (mut is_handled, mut handler_option) = (false, None);
                            for handler in class_and_method.method.get_exception_handlers().0 {
                                let can_handle = match handler.catch_type {
                                    Some(ref class_name) => {
                                        self.check_if_subclass_of(class_name.as_str(), thrown_class_name.as_str())?
                                    }
                                    None => true
                                };
                                if can_handle{
                                    is_handled = true;
                                    handler_option = Some(handler);
                                    break;
                                }
                            }
                            if is_handled{
                                let frame = self.init_call_stack.frames.last_mut().unwrap();
                                frame.pc = handler_option.unwrap().handler_pc;
                                frame.stack.push(throwable);
                                debug!("Exception thrown handled by {}", frame.class_and_method.method.name.as_str());
                            } else {
                                debug!("Exception handler not in this function");
                                return Ok(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)), throwable));
                            }
                        }
                    }
                    VMResultType::ExceptionThrown(error, throwable) => {
                        if let VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)) = &error{
                            let mut frame = self.init_call_stack.pop_call_frame();
                            debug!("Exception thrown by {}: {}: {}", class_and_method.format(), thrown_class_name, message);
                            if class_and_method.method.has_exception_handler() {
                                todo!()
                            } else {
                                return Ok(VMResultType::ExceptionThrown(error, throwable));
                            }
                        } else {
                            unreachable!("Could not handle {} thrown by a function", error);
                        }
                    }
                }
            }
        } else {
            let object = if class_and_method.method.is_static() {
                None
            } else {
                match call_frame.locals.get(0) {
                    Some(local) => {
                        Some(local.expect_reference()?)
                    },
                    None => None
                }
            };
            let args = call_frame.locals
                .iter()
                .cloned()
                .skip(if object.is_none() {0} else {1})
                .take_while(|value| value != &Value::Uninitialized)
                .collect::<Vec<_>>();
            let try_native = NativeMethodRegistry::invoke(self, &class_and_method, object, args);
            if let Some(native) = try_native{
                trace!("'Pushing' native method frame {:?}", call_frame);
                //self.call_stack.push_call_frame(call_frame);
                self.init_call_stack.print_call_stack();
                //self.handle_result(native?)
                trace!("native result is {:?}", native);
                match native? {
                    VMResultType::Ok(optional_value) => {
                        Ok(VMResultType::Ok(optional_value))
                    }
                    VMResultType::CallPaused(new_frame) => {
                        let result = self.invoke_frame(new_frame)?;
                        //should always return VMResultType::Ok(None)
                        assert!(result.is_ok());
                        /*if let VMResultType::Ok(returned_value) = result {
                            self.call_stack.add_to_top_stack(returned_value);
                        }*/
                        Ok(result)
                    }
                    VMResultType::ExceptionThrown(error, throwable) => {
                        Err(error)
                    }
                }
            } else {
                if class_and_method.method.descriptor.return_type.is_some(){
                    error!("Native Method {} wont get executed but return value is probably expected", class_and_method.format());
                    Err(VmError::MethodCallError(format!("Native {}", class_and_method.format())))
                } else {
                    info!("Native Method {} wont get executed", class_and_method.format());
                    Ok(VMResultType::Ok(None))
                    //Err(VmError::MethodCallError(format!("void Native {}", class_and_method.format())))
                }
            }
        }
    }

    pub fn get_or_resolve_class(&mut self, class_name: &str) -> VMResult<ClassRef<'a>>{
        let resolved = self.class_manager.get_or_resolve_class(class_name)?;
        //FIXME maybe make this global
        if let ResolvedClass::NewClass(to_init) = &resolved{
            for class in to_init.to_initialize.iter(){
                if let Some(clinit) = self.init_class(class)?{
                    self.invoke_frame_on_stack(clinit)?;
                }
            }
        }
        Ok(resolved.get_class())
    }

    fn init_class(&mut self, class: ClassRef<'a>) -> VMResult<Option<CallFrame<'a>>>{
        if class.transitive_field_count > 0{
            let static_object = self.new_object_from_class(class);
            self.static_class_objects.insert(class.id, static_object);
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                return Ok(Some(CallStack::create_call_frame(class_and_method, Some(static_object), Vec::new())?));
            }
        }

        Ok(None)
    }

    pub fn resolve_class_method(&mut self, class_name: &str, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        self.get_or_resolve_class(class_name)
            .and_then(|class| {
                class
                    .find_method(method_name, descriptor)
                    .map(|method| ClassAndMethod{ class, method})
                    .ok_or(VmError::JavaException(JavaError::MethodNotFoundException(method_name.to_string())))
            })
    }

    pub fn new_object(&mut self, class_name: &str) -> VMResult<Reference<'a>>{
        let class = self.get_or_resolve_class(class_name)?;
        Ok(self.new_object_from_class(class))
    }

    pub fn new_object_from_class(&self, class: ClassRef<'a>) -> Reference<'a>{
        info!("CC[{:?}] = {}", class.id, class.name);
        self.object_allocator.allocate_object(class)
    }

    pub fn get_static_class_object(&self, id: ClassId) -> Option<Reference<'a>>{
        self.static_class_objects.get(&id).cloned()
    }

    pub fn new_array(&mut self, dims: usize, field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMResult<Reference<'a>>{
        let class_name = match field_type.clone(){
            FieldType::Object(class_name) => {
                "[L".to_string() + &class_name + ";"
            }
            FieldType::Primitive(primitive_type) => {
                "[".to_string() + match primitive_type {
                    PrimitiveType::Boolean => "Z",
                    PrimitiveType::Byte => "B",
                    PrimitiveType::Char => "C",
                    PrimitiveType::Short => "S",
                    PrimitiveType::Integer => "I",
                    PrimitiveType::Long => "J",
                    PrimitiveType::Float => "F",
                    PrimitiveType::Double => "D",
                }
            }
        };
        let class = self.get_or_resolve_class(class_name.as_str())?;
        Ok(self.object_allocator.allocate_array(class, dims, field_type, content))
    }

    pub fn new_string_object(&mut self, string: String) -> VMResult<Reference<'a>>{
        let char_array: Vec<Value<'a>> = string.chars().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);
        let char_array = Value::Reference(self.new_array(1, FieldType::Primitive(PrimitiveType::Char), char_array)?);

        let string_object = self.new_object("java/lang/String")?;
        string_object.set_field(0, char_array);
        string_object.set_field(1, Value::Integer(0));
        string_object.set_field(6, Value::Integer(0));

        if !self.string_objects.contains_key(&string){
            self.string_objects.insert(string, string_object);
            Ok(string_object)
        } else {
            Ok(self.string_objects[&string])
        }

    }

    pub fn extract_string_from_object(value: &Value<'a>) -> VMResult<String>{
        if let Value::Reference(reference) = value{
            let chars = reference.get_field(0);
            if let Value::Reference(char_ref) = chars {
                if let ReferenceType::Array(_, _, content) = &char_ref.reference_type{
                    let chars: Vec<u8> = content.borrow().iter().map(|v| if let Value::Integer(val) = v {*val as u8} else {0}).collect();
                    let string = from_java_cesu8(chars.as_slice())?.to_string();
                    return Ok(string);
                }
            }
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }

    pub fn new_class_object(&mut self, class_name: String) -> VMResult<Reference<'a>>{
        let class = self.get_or_resolve_class(class_name.as_str())?;
        let class_id = class.id;

        if !self.class_objects.contains_key(&class_id){
            let class_object = self.new_object("java/lang/Class")?;
            let string_object = self.new_string_object(class_name)?;

            class_object.set_field(5, Value::Reference(string_object));

            self.class_objects.insert(class_id, class_object);
            Ok(class_object)
        } else {
            Ok(self.class_objects[&class_id])
        }
    }

    pub fn extract_class_from_class_object(&mut self, object: Reference<'a>) -> VMResult<ClassRef<'a>>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        let class = self.get_or_resolve_class(name.as_str())?;
        Ok(class)
    }
    
    pub fn check_if_subclass_of(&mut self, class_name: &str, of_name: &str) -> VMResult<bool>{
        let mut current_class = self.get_or_resolve_class(of_name)?;
        loop {
            if current_class.name == class_name {
                return Ok(true);
            }
            if let Some(super_class) = current_class.superclass {
                current_class = super_class;
            } else {
                return Ok(false);
            }
        }
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_id(class_id)
    }

    pub fn find_class_by_name(&self, name: String) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_name(name.as_str())
    }
}

#[derive(Error, Debug, Clone)]
pub enum VmError{
    #[error("{0}")]
    JavaException(#[from] JavaError),
    #[error("")]
    ParseError(#[from] ClassParseError),
    #[error("Methodcall to {0} failed")]
    MethodCallError(String),
    #[error("Validation failed: expected: {0}")]
    ValidationError(String),
    #[error("{0}")]
    CESU8Error(#[from] Cesu8DecodingError),
}