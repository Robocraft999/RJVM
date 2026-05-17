use callstack::CallStack;
use cesu8::{from_java_cesu8, to_java_cesu8, Cesu8DecodingError};
use log::{debug, error, info, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use std::str::Utf8Error;
use thiserror::Error;

use crate::access_flags::MethodFlag;
use crate::bytecode::Instruction;
use crate::class_file::constant_pool::{BytecodeBehavior, ConstantPoolEntry};
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::attributes::ExceptionTableEntry;
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::error::ClassParseError;
use crate::vm::bytecode::InstructionBlock;
use crate::vm::call_frame::CallFrame;
use crate::vm::class::{Class, ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::{ClassLoadingState, ResolvedClass};
use crate::vm::debug::DebugHelper;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::java_native_method_impl::{register_all_natives, NativeMethodRegistry};
use crate::vm::jni::types::JavaVM;
use crate::vm::r#unsafe::Unsafe;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType};
use crate::{get_or_init, get_or_init_special};
use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;
use crate::class_file::fields::primitive_to_wrapper_name;

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
pub mod result;
pub mod bytecode; //TODO move out from vm
mod executor;
mod debug;
pub mod jni;
mod call_info;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub objects_by_id: RefCell<HashMap<u32, Reference<'a>>>,
    pub static_class_objects: RefCell<HashMap<ClassId, Reference<'a>>>,
    pub string_objects: RefCell<HashMap<String, Reference<'a>>>,
    pub class_objects: RefCell<HashMap<ClassId, Reference<'a>>>,
    pub object_payloads: RefCell<HashMap<u32, Vec<Value<'a>>>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub currently_open_files: RefCell<HashMap<String, (Vec<u8>, usize)>>,
    pub current_thread: RefCell<Option<Reference<'a>>>,
    pub caught_exception: RefCell<Option<(String, String, Value<'a>)>>,
    pub debug_helper: DebugHelper,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let mut class_manager = ClassManager::new(class_path);
        let mut native_method_registry = NativeMethodRegistry::new();
        register_all_natives(&mut native_method_registry);
        let unsafe_allocator = Unsafe::new();
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            unsafe_allocator,
            call_stack: CallStack::new(),
            objects_by_id: RefCell::new(HashMap::new()),
            static_class_objects: RefCell::new(HashMap::new()),
            string_objects: RefCell::new(HashMap::new()),
            class_objects: RefCell::new(HashMap::new()),
            object_payloads: RefCell::new(HashMap::new()),
            native_method_registry,
            currently_open_files: RefCell::new(HashMap::new()),
            current_thread: RefCell::new(None),
            caught_exception: RefCell::new(None),
            debug_helper: DebugHelper::new()
        }
    }

    pub fn dump_class_file(&mut self, class_name: &str) -> VMResult<()>{
        let class = self.get_or_resolve_class(class_name)?;
        info!("Class: {:?}", class);
        Ok(())
    }

    pub fn invoke_new_frame(&self, java_vm: &JavaVM, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
        let current_index = self.call_stack.len() as isize -1;
        self.call_stack.create_and_push_call_frame(class_and_method, object, args, false);
        self.invoke_frames_until(java_vm, current_index)
    }

    /// Returns only Err() or Ok(Successful())
    pub fn invoke_frames_until(&self, java_vm: &JavaVM, stop_index: isize) -> VMPartialResult<Option<Value<'a>>> {
        loop {
            let frame_amount = self.call_stack.len();

            // if an exception is caught, try to let the current frame handle it
            let mut clear_exception = false;
            if let Some((message, origin, throwable)) = self.caught_exception.borrow().as_ref(){
                let thrown_class_name = throwable.expect_reference().map(|r| r.class_name.clone())?;
                if frame_amount as isize - 1 == stop_index{
                    self.debug_helper.exception_helper.push(format!("Subroutine could not handle {} thrown by function {} with message: {}", thrown_class_name, origin, message));
                    return Err(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message.to_owned(), origin.to_owned())));
                }

                let class_and_method = self.call_stack.get_class_and_method_cloned();
                if class_and_method.method.is_native(){
                    self.call_stack.pop_call_frame();
                    debug!("Exception handler not in this native function {}", class_and_method.format());
                    continue;
                }
                let current_pc = &self.call_stack.get_pc();
                //[unchecked] class already loaded by method
                if let Some(handler_pc) = class_and_method.resolve_exception_handler(self, current_pc, thrown_class_name.as_str()){
                    self.call_stack.set_pc(handler_pc);
                    self.call_stack.push_operand_value(throwable.clone());
                    self.debug_helper.exception_helper.push(format!("Handled {} by {}\n└-- thrown by {} with message: {}", thrown_class_name, class_and_method.format(), origin, message));
                    debug!("Exception thrown handled by {}", class_and_method.format());
                    clear_exception = true;
                } else {
                    self.call_stack.pop_call_frame();
                    debug!("Exception handler not in this function {}", class_and_method.format());
                    continue;
                }
            }

            let class_and_method = self.call_stack.get_class_and_method_cloned();
            if clear_exception {
                self.caught_exception.replace(None);
            }

            let call_result = if class_and_method.method.is_native(){
                self.execute_native(java_vm, class_and_method)?
            } else {
                executor::execute(self, java_vm)?
            };

            match call_result {
                // borde alltid och bara vara på return av non-native och native funktioner
                // så den här frame är alltid den översta
                VMResultType::Successful(result) => {
                    let frame = self.call_stack.pop_call_frame();
                    if frame_amount as isize -2 == stop_index{
                        return Ok(VMResultType::Successful(result));
                    }
                    if let Some(value) = result{
                        if frame.should_push_return{
                            self.call_stack.push_operand_value(value);
                        }
                    }
                }
                // returned by both non-native and native functions
                VMResultType::ExceptionThrown => {
                    // thrown exception should be in self.caught_exception
                    // nothing more to do here
                    continue;
                }
                // should only be returned by non-native functions
                VMResultType::Interrupted(frame_amount, reset_pc) => {
                    if reset_pc{
                        let last_frame_index = self.call_stack.pcs.borrow().len() - frame_amount - 1;
                        let current_pc = self.call_stack.pcs.borrow()[last_frame_index];
                        let previous_pc = self.call_stack.frames.borrow()[last_frame_index].class_and_method.method.previous_pc(current_pc);
                        *self.call_stack.pcs.borrow_mut().get_mut(last_frame_index).unwrap() = ProgramCounter(previous_pc);
                    }
                }
            }
        }
    }

    fn execute_native(&self, java_vm: &JavaVM, class_and_method: ClassAndMethod<'a>) -> VMPartialResult<Option<Value<'a>>> {
        //let call_frame = self.call_stack.pop_call_frame();
        
        let object = if class_and_method.method.is_static() {
            None
        } else {
            match self.call_stack.load_local(0) {
                Some(local) => {
                    Some(local.expect_reference()?)
                },
                None => None
            }
        };
        let args = self.call_stack.locals_stack.borrow().last().unwrap()
            .iter()
            .cloned()
            .skip(if object.is_none() {0} else {1})
            .take_while(|value| value != &Value::Uninitialized)
            .collect::<Vec<_>>();
        let try_native = NativeMethodRegistry::invoke(self, java_vm, &class_and_method, object, args);
        debug!("TTT native[{}] returned: {:?}", class_and_method.format(), try_native);
        if let Some(native) = try_native {
            native
        } else {
            debug!("native not found");
            if class_and_method.method.descriptor.return_type.is_some(){
                Err(VmError::MethodCallError(format!("native {} returns a value which is probably used", class_and_method.format())))
            } else {
                warn!(target: "native", "Native function: {} not found. Skipping", class_and_method.format());
                Ok(VMResultType::Successful(None))
            }
        }
    }

    pub fn get_or_resolve_class(&self, class_name: &str) -> VMResult<ClassRef<'a>>{
        let resolved = self.class_manager.get_or_resolve_class(self, class_name)?;
        Ok(resolved)
    }

    pub fn get_or_initialize_class(&self, class_name: &str) -> VMPartialResult<ClassRef<'a>>{
        let resolved = self.get_or_resolve_class(class_name)?;
        let to_init = self.class_manager.get_classes_to_initialize(resolved)?;
        if to_init.len() > 0{
            let count = to_init.iter()
                .map(|clazz| {
                    self.class_manager.update_class_state(clazz, ClassLoadingState::INITIALIZING);
                    self.init_class(clazz)
                })
                .filter(Option::is_some)
                .count();
            if count > 0{
                Ok(VMResultType::Interrupted(count, true))
            } else {
                Ok(VMResultType::Successful(resolved))
            }
        } else {
            Ok(VMResultType::Successful(resolved))
        }
    }

    pub fn try_get_class(&self, class_name: &str) -> VMResult<ClassRef<'a>>{
        self.find_class_by_name(class_name).ok_or(VmError::ClassNotLoadedError(format!("[try_get_class]: Class not loaded: {}", class_name)))
    }

    pub fn define_class(&self, class_name: &str, bytes: Vec<u8>) -> VMPartialResult<Reference<'a>>{
        println!("FIXME: define_class");
        let resolved = match self.find_class_by_name(class_name){
            Some(resolved) => resolved,
            None => self.class_manager.parse_and_load_class(self, class_name, class_name, None, bytes)?
        };
        self.new_class_object_by_class(resolved)
    }

    fn init_class(&self, class: ClassRef<'a>) -> Option<()>{
        info!("IC[{}]", class.name);
        if class.transitive_field_count > 0 && !class.is_array(){
            let static_object = self.new_object_from_class(class);
            self.static_class_objects.borrow_mut().insert(class.id, static_object);
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                self.call_stack.create_and_push_call_frame(class_and_method, Some(static_object), Vec::new(), false);
                return Some(());
            }
        }
        self.class_manager.update_class_state(class, ClassLoadingState::INITIALIZED);
        None
    }

    pub fn resolve_class_method(&self, class_name: &str, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        let result = self.get_or_resolve_class(class_name);
        result.and_then(|class| {
            self.get_class_method(class, method_name, descriptor)
        })
    }
    
    pub fn get_class_method(&self, class: ClassRef<'a>, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        class
            .find_method(method_name, descriptor)
            .map(|method| ClassAndMethod{ class, method})
            .ok_or(VmError::JavaException(JavaError::MethodNotFoundException(method_name.to_string())))
    }

    pub fn new_object(&self, class_name: &str) -> VMPartialResult<Reference<'a>>{
        get_or_init_special!(self.get_or_initialize_class(class_name)?, |class| Ok(VMResultType::Successful(self.new_object_from_class(class))))
    }

    pub fn try_new_object(&self, class_name: &str) -> VMResult<Reference<'a>>{
        let result = self.new_object(class_name)?;
        if let VMResultType::Successful(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError(format!("[try_new_object]: Class not loaded: {}", class_name)))
        }
    }

    pub fn new_object_from_class(&self, class: ClassRef<'a>) -> Reference<'a>{
        info!("CC[{:?}] = {}", class.id, class.name);
        let fields = class.get_fields(&self);
        let obj = self.object_allocator.allocate_object(class, fields);
        self.objects_by_id.borrow_mut().insert(obj.id, obj);
        self.debug_helper.tracker.push_object_event(obj.id, format!("Object ({}) allocated in {:?}", class.name, self.call_stack.frames.borrow().last().map(|f| f.class_and_method.format())));
        obj
    }

    pub fn get_static_class_object(&self, id: ClassId) -> Option<Reference<'a>>{
        self.static_class_objects.borrow().get(&id).cloned()
    }

    pub fn new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMPartialResult<Reference<'a>>{
        let (class_name, component_type) = if let FieldType::Array(class_name, component_type) = array_field_type {
            (class_name, component_type)
        } else {
            unreachable!("The field type for creating an array has to be an array field type")
        };
        //FIXME verify if this is correct / maybe have to init the component type
        let class = self.get_or_resolve_class(class_name.as_str())?;
        let obj = self.object_allocator.allocate_array(class, dims, *component_type, content);
        self.objects_by_id.borrow_mut().insert(obj.id, obj);
        self.debug_helper.tracker.push_object_event(obj.id, format!("Array allocated:   \n{:?}", obj));
        Ok(VMResultType::Successful(obj))
        /*get_or_init_special!(self.get_or_initialize_class(class_name.as_str())?,
            |class| {
                let obj = self.object_allocator.allocate_array(class, dims, *component_type, content);
                self.objects_by_id.borrow_mut().insert(obj.id, obj);
                self.debug_helper.tracker.push_object_event(obj.id, format!("Array ({}) allocated", class.name));
                Ok(VMResultType::Successful(obj))
            }
        )*/
    }

    pub fn try_new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMResult<Reference<'a>>{
        let result = self.new_array(dims, array_field_type, content)?;
        if let VMResultType::Successful(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_object]: Class not loaded".to_string()))
        }
    }
    
    pub fn new_class_array_1(&self, content: Vec<Value<'a>>) -> VMPartialResult<Reference<'a>>{
        self.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(content))
    }

    pub fn new_object_array_1(&self, content: Vec<Value<'a>>) -> VMPartialResult<Reference<'a>>{
        self.new_array(1, FieldType::Object("java/lang/Object".to_string()).to_array_field_type(1), RefCell::new(content))
    }

    pub fn try_new_string_object(&self, string: &str) -> VMResult<Reference<'a>>{
        let result = self.new_string_object(string)?;
        if let VMResultType::Successful(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_string_object]: Class not loaded".to_string()))
        }
    }

    pub fn try_new_class_object(&self, class: ClassRef<'a>) -> VMResult<Reference<'a>>{
        let result = self.new_class_object_by_class(class)?;
        if let VMResultType::Successful(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_class_object]: Class not loaded".to_string()))
        }
    }
    
    pub fn new_string_object(&self, string: &str) -> VMPartialResult<Reference<'a>>{
        if self.string_objects.borrow().contains_key(string){
            return Ok(VMResultType::Successful(self.string_objects.borrow()[string]))
        }
        
        let char_array: Vec<Value<'a>> = string.chars().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);

        let char_array = Value::Reference(get_or_init!(self.new_array(1, FieldType::Primitive(PrimitiveType::Char).to_array_field_type(1), char_array)?));

        let string_object = get_or_init!(self.new_object("java/lang/String")?);

        //value
        string_object.set_field(0, char_array);
        //hash
        string_object.set_field(1, Value::Integer(0));

        self.string_objects.borrow_mut().insert(string.to_owned(), string_object);
        Ok(VMResultType::Successful(string_object))
    }

    pub fn extract_string_from_object(value: &Value<'a>) -> VMResult<String>{
        if let Value::Reference(reference) = value{
            if !reference.is_null() {
                let chars = reference.get_field(0);
                return Self::extract_string_from_char_arr(&chars);
            }
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }
    
    pub fn extract_string_from_char_arr(chars: &Value<'a>) -> VMResult<String>{
        if let Value::Reference(char_ref) = chars {
            if let ReferenceType::Array(_, _, content) = &char_ref.reference_type{
                let chars: Vec<u8> = content.borrow().iter().map(|v| if let Value::Integer(val) = v {*val as u8} else {0}).collect();
                let string = from_java_cesu8(chars.as_slice())?.to_string();
                return Ok(string);
            }
        }
        Err(VmError::ValidationError(format!( "Expected CharArray but found: {:?}", chars)))
    }

    // FIXME use only ClassRef instead
    fn new_class_object(&self, class_name: &str, class_id: ClassId) -> VMPartialResult<Reference<'a>>{
        if !self.class_objects.borrow().contains_key(&class_id){
            let class_object = get_or_init!(self.new_object("java/lang/Class")?);
            let string_object = get_or_init!(self.new_string_object(class_name.replace("/", ".").as_str())?);

            //name
            class_object.set_field(5, Value::Reference(string_object));

            self.class_objects.borrow_mut().insert(class_id, class_object);
            Ok(VMResultType::Successful(class_object))
        } else {
            Ok(VMResultType::Successful(self.class_objects.borrow()[&class_id]))
        }
    }

    pub fn new_class_object_by_name(&self, class_name: &str) -> VMPartialResult<Reference<'a>> {
        let class = self.get_or_resolve_class(class_name)?;
        self.new_class_object(class_name, class.id)
    }

    pub fn new_class_object_by_class(&self, class: ClassRef<'a>) -> VMPartialResult<Reference<'a>> {
        self.new_class_object(class.name.as_str(), class.id)
    }

    pub fn new_class_object_from_field_type(&self, field_type: &FieldType) -> VMPartialResult<Reference<'a>> {
        let class_name = field_type.to_class_name();
        if field_type.is_primitive() {
            let class_id = self.class_manager.get_primitive_class(self, class_name.as_str());
            self.new_class_object(class_name.as_str(), class_id)
        } else {
            self.new_class_object_by_name(class_name.as_str())
        }
    }

    pub fn new_method_type(&self, java_vm: &JavaVM, descriptor: &MethodDescriptor) -> VMPartialResult<Option<Value<'a>>> {
        let method_type_class = get_or_init!(self.get_or_initialize_class("java/lang/invoke/MethodType")?);

        let mut b_args_classes = Vec::new();
        for ft in &descriptor.args{
            /*let class_name = if ft.is_primitive() {
                primitive_to_wrapper_name(ft.to_class_name().as_str())
            } else {
                ft.to_class_name()
            };
            let class_ref = get_or_init!(self.new_class_object_by_name(class_name.as_str())?);*/
            let class_ref = get_or_init!(self.new_class_object_from_field_type(ft)?);
            b_args_classes.push(Value::Reference(class_ref));
        }
        let b_ret_type_class_name = descriptor.return_type.clone();
        let b_ret_type = if let Some(ft) = b_ret_type_class_name{
            /*let class_name = if ft.is_primitive() {
                primitive_to_wrapper_name(ft.to_class_name().as_str())
            } else {
                ft.to_class_name()
            };
            Value::Reference(get_or_init!(self.new_class_object_by_name(class_name.as_str())?))*/
            Value::Reference(get_or_init!(self.new_class_object_from_field_type(&ft)?))
        } else {
            self.null()
        };
        let b_args_arr = Value::Reference(get_or_init!(self.new_class_array_1(b_args_classes)?));
        let helper = self.resolve_class_method(
            "java/lang/invoke/MethodHandleNatives",
            "findMethodHandleType",
            "(Ljava/lang/Class;[Ljava/lang/Class;)Ljava/lang/invoke/MethodType;"
        ).unwrap();
        let frame_index = self.call_stack.len() as isize - 1;
        self.call_stack.create_and_push_call_frame(helper, None, vec![b_ret_type, b_args_arr], false);
        self.invoke_frames_until(java_vm, frame_index)
    }
    
    /// does call a function which places the result in the current frame
    pub fn new_method_handle(&self, java_vm: &JavaVM, pool_holder: ClassRef<'a>, kind: BytecodeBehavior, cam: ClassAndMethod, method_type_ref: Reference<'a>) -> VMPartialResult<Option<Value<'a>>>{
        let callee = get_or_init!(self.get_or_initialize_class(cam.class.name.as_str())?);
        let callee = Value::Reference(get_or_init!(self.new_class_object_by_class(callee)?));
        let caller = Value::Reference(get_or_init!(self.new_class_object_by_class(pool_holder)?));
        let ref_kind = Value::Integer(kind as u8 as i32);
        let name = Value::Reference(get_or_init!(self.new_string_object(cam.method.name.as_str())?));
        let typ = Value::Reference(method_type_ref);
        let helper = self.resolve_class_method(
            "java/lang/invoke/MethodHandleNatives",
            "linkMethodHandleConstant",
            "(Ljava/lang/Class;ILjava/lang/Class;Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/invoke/MethodHandle;"
        ).unwrap();
        println!("NMH e");
        let frame_index = self.call_stack.len() as isize - 1;
        self.call_stack.create_and_push_call_frame(helper, None, vec![caller, ref_kind, callee, name, typ], false);
        self.invoke_frames_until(java_vm, frame_index)
    }

    pub fn new_java_lang_long(&self, value: Value<'a>) -> VMPartialResult<Value<'a>> {
        let long = get_or_init!(self.new_object("java/lang/Long")?);
        //value
        long.set_field(4, value);
        Ok(VMResultType::Successful(Value::Reference(long)))
    }

    pub fn extract_long(&self, value: Value<'a>) -> VMResult<Value<'a>> {
        if let Value::Reference(long_ref) = value && long_ref.class_name == "java/lang/Long" {
            let value = long_ref.get_field(4).expect_long()?;
            Ok(Value::Long(value))
        } else {
            Err(VmError::ValidationError("expected a long reference".to_string()))
        }
    }

    pub fn extract_class_from_class_object(&self, object: Reference<'a>) -> VMResult<ClassRef<'a>>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        let class = self.get_or_resolve_class(name.as_str())?;

        Ok(class)
    }
    
    pub fn extract_class_name_from_class_object(object: Reference<'a>) -> VMResult<String>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        Ok(name)
    }

    pub fn unchecked_check_if_subclass_of(&self, class_name: &str, of_name: &str) -> VMResult<bool>{
        let mut current_class = self.try_get_class(of_name)?;
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

    pub fn is_instance_of(&self, class: ClassRef<'a>, of_class: ClassRef<'a>) -> bool{
        let mut instance_of = false;
        let mut to_check = vec![class];
        while let Some(next_class) = to_check.pop() {
            if next_class.id == of_class.id{
                instance_of = true;
                break;
            }
            if let Some(super_class) = next_class.superclass{
                to_check.push(super_class);
            }
            to_check.extend(next_class.interfaces.iter());
        }
        instance_of
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_id(class_id)
    }

    pub fn find_class_by_name(&self, name: &str) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_name(name)
    }

    pub fn null(&self) -> Value<'a>{
        Value::Reference(self.object_allocator.null)
    }

    /// Returns a `VMResultType::ExceptionThrown` and places the throwable into the exception slot
    ///
    /// `throwable_class` has to be initialized beforehand
    ///
    pub fn throw<T>(&self, throwable_class: ClassRef<'a>, message: String, origin: String) -> VMPartialResult<T> {
        //let exception_class = self.get_or_initialize_class(&throwable_class_name)?;
        let exception_object = self.new_object_from_class(throwable_class);

        let details = self.try_new_string_object(message.as_str())?;
        //detailsMessage
        exception_object.set_field(2, Value::Reference(details));

        let prev = self.caught_exception.replace(
            Some((
                message,
                origin,
                Value::Reference(exception_object)
            )));
        assert!(prev.is_none());
        Ok(VMResultType::ExceptionThrown)
    }
}

impl !Unpin for VM<'_>{}

impl Drop for VM<'_>{
    fn drop(&mut self) {
        error!("VM drop: {:p}", self);
    }
}

fn successful_result<T>(res: T) -> VMPartialResult<T> {
    Ok(VMResultType::Successful(res))
}

#[derive(Error, Debug, Clone)]
pub enum VmError{
    #[error("{0}")]
    JavaException(#[from] JavaError),

    #[error("{0}")]
    ParseError(#[from] ClassParseError),

    #[error("{0}")]
    NomError(#[from] nom::Err<nom::error::Error<&'static [u8]>>),

    #[error("Methodcall to {0} failed")]
    MethodCallError(String),

    #[error("Validation failed: expected: {0}")]
    ValidationError(String),

    #[error("{0}")]
    CESU8Error(#[from] Cesu8DecodingError),

    #[error("{0}")]
    ClassNotLoadedError(String),

    #[error("{0}")]
    Unspecified(String),

    #[error("Error caught in native function: {0}")]
    Native(String)
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ProgramCounter(pub u16);