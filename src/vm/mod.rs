use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::Utf8Error;
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
use crate::vm::value::{Reference, ReferenceType};

pub mod class_path;
pub mod class_path_entry;
pub mod class_manager;
mod java_error;
pub mod value;
mod call_frame;
mod callstack;
mod class;
mod gc;
mod java_native_method_impl;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub static_class_objects: HashMap<ClassId, Reference<'a>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let class_manager = ClassManager::new(class_path);
        let mut native_method_registry = NativeMethodRegistry::new();
        register_all_natives(&mut native_method_registry);
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            call_stack: CallStack::new(),
            static_class_objects: HashMap::new(),
            native_method_registry,
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

            //self.call_stack.pop_call_frame();

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

        /*if class.name == "java/lang/System"{
            if let Some(setout0_method) = class.find_method("setOut0", "(Ljava/io/PrintStream;)V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: setout0_method,
                };
                let file_descriptor = self.new_object("java/io/FileDescriptor")?;
                let static_file_descriptor = self.get_static_class_object(file_descriptor.id).unwrap();
                //public static final FileDescriptor out = new FileDescriptor(1);
                let file_descriptor_out = static_file_descriptor.get_field(3);
                let file_output_stream = self.new_object("java/io/FileOutputStream")?;
                let file_output_stream_init = self.resolve_class_method("java/io/FileOutputStream", "<init>", "(Ljava/io/FileDescriptor;)V")?;
                self.invoke(file_output_stream_init, Some(file_output_stream), vec![file_descriptor_out])?;

                let buffered_output_stream = self.new_object("java/io/BufferedOutputStream")?;
                let buffered_output_stream_init = self.resolve_class_method("java/io/BufferedOutputStream", "<init>", "(Ljava/io/OutputStream;I)V")?;
                self.invoke(buffered_output_stream_init, Some(buffered_output_stream), vec![Value::Object(file_output_stream), Value::Integer(128)])?;

                let print_stream = self.new_object("java/io/PrintStream")?;
                let print_stream_init = self.resolve_class_method("java/io/PrintStream", "<init>", "(Ljava/io/OutputStream;I)V")?;
                self.invoke(print_stream_init, Some(print_stream), vec![Value::Object(buffered_output_stream), Value::Integer(1)])?;

                self.invoke(class_and_method, Some(static_object), vec![Value::Object(print_stream)])?;
            }
        }*/
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
        let class_name = field_type.to_class_name();
        let class = self.get_or_resolve_class(class_name.as_str())?;
        Ok(self.object_allocator.allocate_array(class, dims, field_type, content))
    }

    pub fn new_string_object(&mut self, string: String) -> Result<Reference<'a>, VmError>{
        let char_array: Vec<Value<'a>> = string.encode_utf16().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);
        let char_array = Value::Reference(self.new_array(1, FieldType::Primitive(PrimitiveType::Char), char_array)?);

        let string_object = self.new_object("java/lang/String")?;
        string_object.set_field(0, char_array);
        string_object.set_field(1, Value::Integer(0));
        string_object.set_field(6, Value::Integer(0));

        Ok(string_object)
    }

    pub fn extract_string_from_object(&self, value: &Value<'a>) -> Result<String, VmError>{
        if let Value::Reference(reference) = value{
            let chars = reference.get_field(0);
            if let Value::Reference(char_ref) = chars {
                if let ReferenceType::Array(_, _, content) = &char_ref.reference_type{
                    let chars: Vec<u8> = content.borrow().iter().map(|v| if let Value::Integer(val) = v {*val as u8} else {0}).collect();
                    let string = String::from_utf8(chars).map_err(|e| e.utf8_error())?;
                    debug!("string from object: {:?}", string);
                    return Ok(string);
                }
            }
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }

    pub fn new_class_object(&mut self, class_name: String) -> Result<Reference<'a>, VmError>{
        let class_object = self.new_object("java/lang/Class")?;
        let string_object = self.new_string_object(class_name)?;

        class_object.set_field(5, Value::Reference(string_object));
        Ok(class_object)
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_id(class_id)
    }
}

#[derive(Error, Debug, PartialEq)]
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
    UTF8Error(#[from] Utf8Error),
}