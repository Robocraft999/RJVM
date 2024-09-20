use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::Utf8Error;
use cesu8::{from_java_cesu8, to_java_cesu8, Cesu8DecodingError};
use callstack::CallStack;
use log::{debug, error, info};
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

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub static_class_objects: HashMap<ClassId, Reference<'a>>,
    pub string_objects: HashMap<String, Reference<'a>>,
    pub class_objects: HashMap<ClassId, Reference<'a>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub current_thread: Option<Reference<'a>>
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
            current_thread: None
        }
    }

    pub fn invoke(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
        if !class_and_method.method.is_native(){
            let method_signature = format!("{}.{}{}", class_and_method.class.name, class_and_method.method.name, class_and_method.method.descriptor.as_str());
            info!("INVOKE {} on {:?} with {:?}", method_signature, object, args);
            
            /*let mut frame = */self.call_stack.push_call_frame(class_and_method, object, args)?;
            self.call_stack.print_call_stack();
            let vm_ptr: *mut VM = self;
            //let result = frame.execute(self)?;
            let result = self.call_stack.execute_top(vm_ptr)?;
            
            info!("INVRETURN {} returned: {:?}", method_signature, result);

            self.call_stack.pop_call_frame();

            Ok(result)
        } else {
            let class_name = class_and_method.class.name.clone();
            let method_name = class_and_method.method.name.clone();
            let try_native = NativeMethodRegistry::invoke(self, &class_and_method, object, args);
            if let Some(native) = try_native{
                return native;
            } else {
                if class_and_method.method.descriptor.return_type.is_some(){
                    error!("Native Method {}.{} wont get executed but return value is probably expected", class_name, method_name);
                } else {
                    info!("Native Method {}.{} wont get executed", class_name, method_name);
                }

                return Ok(None);
            }
            Err(VmError::JavaException(JavaError::MethodNotFoundException(method_name)))
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
        if class.transitive_field_count > 0{
            let static_object = self.new_object_from_class(class);
            self.static_class_objects.insert(class.id, static_object);
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                self.invoke(class_and_method, Some(static_object), Vec::new())?;
            }
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

    pub fn new_object(&mut self, class_name: &str) -> Result<Reference<'a>, VmError>{
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

    pub fn new_array(&mut self, dims: usize, field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> Result<Reference<'a>, VmError>{
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

    pub fn new_string_object(&mut self, string: String) -> Result<Reference<'a>, VmError>{
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

    pub fn extract_string_from_object(&self, value: &Value<'a>) -> Result<String, VmError>{
        if let Value::Reference(reference) = value{
            let chars = reference.get_field(0);
            if let Value::Reference(char_ref) = chars {
                if let ReferenceType::Array(_, _, content) = &char_ref.reference_type{
                    let chars: Vec<u8> = content.borrow().iter().map(|v| if let Value::Integer(val) = v {*val as u8} else {0}).collect();
                    let string = from_java_cesu8(chars.as_slice())?.to_string();
                    debug!("string from object: {:?}", string);
                    return Ok(string);
                }
            }
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }

    pub fn new_class_object(&mut self, class_name: String) -> Result<Reference<'a>, VmError>{
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

    pub fn extract_class_from_class_object(&mut self, object: Reference<'a>) -> Result<ClassRef<'a>, VmError>{
        let name_object = object.get_field(5);
        let name = self.extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        let class = self.get_or_resolve_class(name.as_str())?;
        Ok(class)
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_id(class_id)
    }

    pub fn find_class_by_name(&self, name: String) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_name(name.as_str())
    }
}

#[derive(Error, Debug)]
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