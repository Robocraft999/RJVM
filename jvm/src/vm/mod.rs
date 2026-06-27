use callstack::CallStack;
use cesu8::{from_java_cesu8, Cesu8DecodingError};
use log::{debug, error, info, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{PoisonError, RwLock};
use thiserror::Error;

use crate::class_file::constant_pool::BytecodeBehavior;
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::error::ClassParseError;
use crate::vm::class::{ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::ClassLoadingState;
use crate::vm::constants::{CLASS_name_INDEX, LONG_value_INDEX, METHODTYPE_ptypes_INDEX, METHODTYPE_rtype_INDEX, STRING_hash_INDEX, STRING_value_INDEX, THROWABLE_detailsMessage_INDEX};
use crate::vm::debug::DebugHelper;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::jni::types::{jclass, jobject, JavaVM};
use crate::vm::native::{register_all_natives, NativeMethodRegistry};
use crate::vm::r#unsafe::Unsafe;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{RefId, Reference, ReferenceType};
use crate::{get_or_init, get_or_init_special};
use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;
use crate::class_file::fields::get_class_descriptor;
use crate::vm::constants::classes::{JAVA_LANG_CLASS, JAVA_LANG_INVOKE_METHOD_TYPE, JAVA_LANG_INVOKE_MHN, JAVA_LANG_LONG, JAVA_LANG_STRING};
use crate::vm::java_thread::JavaThread;

pub mod class_path;
pub mod class_path_entry;
pub mod class_manager;
mod java_error;
pub mod value;
mod call_frame;
mod callstack;
pub mod class;
mod gc;
mod r#unsafe;
pub mod result;
pub mod bytecode; //TODO move out from vm
mod executor;
mod debug;
pub mod jni;
mod call_info;
pub(crate) mod constants;
mod native;
mod java_thread;
pub mod application;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub objects_by_id: RwLock<HashMap<RefId, Reference<'a>>>,
    pub static_class_objects: RwLock<HashMap<ClassId, Reference<'a>>>,
    pub string_objects: RwLock<HashMap<String, Reference<'a>>>,
    pub class_objects: RwLock<HashMap<ClassId, Reference<'a>>>,
    pub object_payloads: RwLock<HashMap<RefId, Vec<Value>>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub currently_open_files: RwLock<HashMap<String, (Vec<u8>, usize)>>,
    pub current_locks: RwLock<HashMap<RefId, usize>>,
    pub vm_debug_helper: DebugHelper,
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
            objects_by_id: RwLock::new(HashMap::new()),
            static_class_objects: RwLock::new(HashMap::new()),
            string_objects: RwLock::new(HashMap::new()),
            class_objects: RwLock::new(HashMap::new()),
            object_payloads: RwLock::new(HashMap::new()),
            native_method_registry,
            currently_open_files: RwLock::new(HashMap::new()),
            current_locks: RwLock::new(HashMap::new()),
            vm_debug_helper: DebugHelper::new(),
        }
    }

    pub fn dump_class_file(&mut self, class_name: &str) -> VMResult<()>{
        let class = self.get_or_resolve_class(class_name)?;
        info!("Class: {:?}", class);
        Ok(())
    }

    pub fn get_or_resolve_class(&self, class_name: &str) -> VMResult<ClassRef<'a>>{
        let resolved = self.class_manager.get_or_resolve_class(self, class_name)?;
        Ok(resolved)
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

    pub fn new_object_from_class(&self, class: ClassRef<'a>) -> Reference<'a>{
        info!("CC[{:?}] = {}", class.id, class.name);
        let fields = class.get_fields(&self);
        let obj = self.object_allocator.allocate_object(class, fields);
        if let Ok(mut res) = self.objects_by_id.write() {
            res.insert(obj.id, obj);
        } else {
            unreachable!("Object couldn't be allocated")
        }
        self.vm_debug_helper.tracker.push_object_event(obj.id, format!("Object ({})", class.name));
        obj
    }

    pub fn get_static_class_object(&self, id: ClassId) -> Option<Reference<'a>>{
        if let Ok(res) = self.static_class_objects.read() {
            res.get(&id).cloned()
        } else {
            None
        }
    }

    pub fn new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value>>) -> VMPartialResult<Reference<'a>>{
        let (class_name, component_type) = if let FieldType::Array(class_name, component_type) = array_field_type {
            (class_name, component_type)
        } else {
            unreachable!("The field type for creating an array has to be an array field type")
        };
        //FIXME verify if this is correct / maybe have to init the component type
        let class = self.get_or_resolve_class(class_name.as_str())?;
        let obj = self.object_allocator.allocate_array(class, dims, *component_type, content);
        self.objects_by_id.write()?.insert(obj.id, obj);
        self.vm_debug_helper.tracker.push_object_event(obj.id, format!("Array allocated:   \n{:?}", obj));
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

    pub fn try_new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value>>) -> VMResult<Reference<'a>>{
        let result = self.new_array(dims, array_field_type, content)?;
        if let VMResultType::Successful(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_object]: Class not loaded".to_string()))
        }
    }
    
    pub fn new_class_array_1(&self, content: Vec<Value>) -> VMPartialResult<Reference<'a>>{
        self.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(content))
    }

    pub fn new_object_array_1(&self, content: Vec<Value>) -> VMPartialResult<Reference<'a>>{
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
        if self.string_objects.read()?.contains_key(string){
            return Ok(VMResultType::Successful(self.string_objects.read()?[string]))
        }
        
        let char_array: Vec<Value> = string.chars().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);

        let char_array = Value::Reference(get_or_init!(self.new_array(1, FieldType::Primitive(PrimitiveType::Char).to_array_field_type(1), char_array)?).id);

        let string_clazz = self.get_or_resolve_class(JAVA_LANG_STRING)?;
        let string_object = self.new_object_from_class(string_clazz);

        //value
        string_object.set_field(STRING_value_INDEX, char_array);
        //hash
        string_object.set_field(STRING_hash_INDEX, Value::Integer(0));

        self.string_objects.write()?.insert(string.to_owned(), string_object);
        Ok(VMResultType::Successful(string_object))
    }

    pub fn extract_string_from_value(&self, value: Value) -> VMResult<String>{
        if let Value::Reference(ref_id) = value{
            if !ref_id.is_null() {
                let string_ref = self.resolve_object_by_id(ref_id)?;
                let chars = string_ref.get_field(STRING_value_INDEX);
                return self.extract_string_from_char_arr(chars);
            }
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }

    pub fn extract_string_from_ref(&self, string_ref: Reference<'a>) -> VMResult<String> {
        if string_ref.is_null() { return Err(VmError::ValidationError("Expected String Reference but was null".to_string())); }
        let chars = string_ref.get_field(STRING_value_INDEX);
        self.extract_string_from_char_arr(chars)
    }
    
    pub fn extract_string_from_char_arr(&self, chars: Value) -> VMResult<String>{
        if let Value::Reference(char_arr_id) = chars {
            let char_ref = self.resolve_object_by_id(char_arr_id)?;
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
        if !self.class_objects.read()?.contains_key(&class_id){
            let class_clazz = self.get_or_resolve_class(JAVA_LANG_CLASS)?;
            let class_object = self.new_object_from_class(class_clazz);
            let string_object = get_or_init!(self.new_string_object(class_name.replace("/", ".").as_str())?);

            //name
            class_object.set_field(CLASS_name_INDEX, Value::Reference(string_object.id));

            self.class_objects.write()?.insert(class_id, class_object);
            Ok(VMResultType::Successful(class_object))
        } else {
            Ok(VMResultType::Successful(self.class_objects.read()?[&class_id]))
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

    pub fn new_java_lang_long(&self, value: Value) -> VMResult<Value> {
        let long_clazz = self.get_or_resolve_class(JAVA_LANG_LONG)?;
        let long = self.new_object_from_class(long_clazz);
        //value
        long.set_field(LONG_value_INDEX, value);
        Ok(Value::Reference(long.id))
    }

    pub fn extract_long(&self, value: Value) -> VMResult<Value> {
        if let Value::Reference(long_id) = value {
            let long_ref = self.resolve_object_by_id(long_id)?;
            let value = long_ref.get_long_field(LONG_value_INDEX)?;
            Ok(Value::Long(value))
        } else {
            Err(VmError::ValidationError("expected a long reference".to_string()))
        }
    }

    pub fn extract_class_from_class_object(&self, object: Reference<'a>) -> VMResult<ClassRef<'a>>{
        let name_object = object.get_field(CLASS_name_INDEX);
        let name = self.extract_string_from_value(name_object)?;
        let name = name.replace(".", "/");
        let class = self.get_or_resolve_class(name.as_str());
        match class {
            Ok(class) => Ok(class),
            Err(e) => {
                match self.class_manager.anonymous_classes.read()?.get(&object.id) {
                    Some(info) => Ok(info.clazz),
                    None => Err(e),
                }
            }
        }
    }
    
    pub fn extract_class_name_from_class_ref(&self, object: Reference<'a>) -> VMResult<String>{
        let name_object = object.get_field(CLASS_name_INDEX);
        let name = self.extract_string_from_value(name_object)?;
        let name = name.replace(".", "/");
        Ok(name)
    }

    pub fn extract_descriptor_from_method_type(&self, method_type_ref: Reference) -> VMResult<String> {
        let ptypes_array_ref = self.resolve_object_by_id(method_type_ref.get_ref_field(METHODTYPE_ptypes_INDEX)?)?;

        let mut desc = String::from("(");
        if let ReferenceType::Array(_, _, content) = &ptypes_array_ref.reference_type {
            for p in content.borrow().iter() {
                let Value::Reference(param_class_ref_id) = p else { return Err(VmError::ValidationError("Expected a reference".to_string())); };
                let param_class_name = &self.resolve_clazz_by_class_ref_id(*param_class_ref_id)?.name;
                println!("{}", param_class_name);
                desc += get_class_descriptor(param_class_name.as_str()).as_str();
            }
        }
        desc.push_str(")");

        let rtype_ref = self.resolve_object_by_id(method_type_ref.get_ref_field(METHODTYPE_rtype_INDEX)?)?;
        if rtype_ref.is_null() {
            unreachable!("It seems like the return type cant actually be null")
        }
        let rtype = self.extract_class_name_from_class_ref(rtype_ref)?;
        desc += get_class_descriptor(rtype.as_str()).as_str();
        Ok(desc)
    }
    
    // native helpers
    pub fn resolve_class_object_by_jclass(&self, id: jclass) -> ClassRef<'a> {
        let class_ref = self.resolve_object_by_jobject(id).unwrap();
        self.extract_class_from_class_object(&class_ref).unwrap()
    }
    
    pub fn resolve_object_by_jobject(&self, id: jobject) -> Option<Reference<'a>> {
        if let Ok(res) = self.objects_by_id.read() {
            res.get(&RefId(id)).copied()
        } else {
            unreachable!("Could not acquire objects lock")
        }
    }

    // object access
    pub fn resolve_object_by_id(&self, id: RefId) -> VMResult<Reference<'a>> {
        self.objects_by_id.read()?.get(&id).copied().ok_or(VmError::ValidationError(format!("Object not found: {:?}", id)))
    }

    pub fn resolve_clazz_by_class_ref_id(&self, ref_id: RefId) -> VMResult<ClassRef<'a>> {
        let class_ref = self.resolve_object_by_id(ref_id)?;
        self.extract_class_from_class_object(&class_ref)
    }
    
    // util

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

    pub fn null(&self) -> Value{
        Value::Reference(RefId(0))
    }

    pub fn null_ref(&self) -> Reference<'a> {
        self.object_allocator.null
    }
}

impl !Unpin for VM<'_>{}

impl Drop for VM<'_>{
    fn drop(&mut self) {
        error!("VM drop: {:p}", self);
    }
}

#[derive(Clone, Copy)]
pub struct Context<'a, 'b> {
    pub thread: &'b JavaThread,
    pub vm: &'b VM<'a>,
    pub java_vm: &'b JavaVM<'a>,
}

impl<'a> Context<'a, '_> {
    pub fn new_method_type(&self, descriptor: &MethodDescriptor) -> VMPartialResult<Option<Value>> {
        let mut b_args_classes = Vec::new();
        for ft in &descriptor.args{
            /*let class_name = if ft.is_primitive() {
                primitive_to_wrapper_name(ft.to_class_name().as_str())
            } else {
                ft.to_class_name()
            };
            let class_ref = get_or_init!(self.new_class_object_by_name(class_name.as_str())?);*/
            let class_ref = get_or_init!(self.vm.new_class_object_from_field_type(ft)?);
            b_args_classes.push(Value::Reference(class_ref.id));
        }
        let b_ret_type_class_name = descriptor.return_type.clone();
        let b_ret_type = if let Some(ft) = b_ret_type_class_name{
            /*let class_name = if ft.is_primitive() {
                primitive_to_wrapper_name(ft.to_class_name().as_str())
            } else {
                ft.to_class_name()
            };
            Value::Reference(get_or_init!(self.new_class_object_by_name(class_name.as_str())?))*/
            Value::Reference(get_or_init!(self.vm.new_class_object_from_field_type(&ft)?).id)
        } else {
            self.vm.null()
        };
        let b_args_arr = Value::Reference(get_or_init!(self.vm.new_class_array_1(b_args_classes)?).id);
        let helper = self.vm.resolve_class_method(
            JAVA_LANG_INVOKE_MHN,
            "findMethodHandleType",
            "(Ljava/lang/Class;[Ljava/lang/Class;)Ljava/lang/invoke/MethodType;"
        ).unwrap();
        JavaThread::invoke_subroutine(*self, helper, None, vec![b_ret_type, b_args_arr])
    }

    pub fn new_method_handle(&self, pool_holder: ClassRef<'a>, kind: BytecodeBehavior, cam: ClassAndMethod<'a>, method_type_ref: Reference<'a>) -> VMPartialResult<Option<Value>>{
        get_or_init!(self.ensure_initialized(cam.class)?);
        let callee = Value::Reference(get_or_init!(self.vm.new_class_object_by_class(cam.class)?).id);
        let caller = Value::Reference(get_or_init!(self.vm.new_class_object_by_class(pool_holder)?).id);
        let ref_kind = Value::Integer(kind as u8 as i32);
        let name = Value::Reference(get_or_init!(self.vm.new_string_object(cam.method.name.as_str())?).id);
        let typ = Value::Reference(method_type_ref.id);
        let helper = self.vm.resolve_class_method(
            JAVA_LANG_INVOKE_MHN,
            "linkMethodHandleConstant",
            "(Ljava/lang/Class;ILjava/lang/Class;Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/invoke/MethodHandle;"
        ).unwrap();
        println!("NMH e");
        JavaThread::invoke_subroutine(*self, helper, None, vec![caller, ref_kind, callee, name, typ])
    }

    pub fn get_or_initialize_class(&self, class_name: &str) -> VMPartialResult<ClassRef<'a>>{
        let resolved = self.vm.get_or_resolve_class(class_name)?;
        self.ensure_initialized(resolved)?;
        successful_result(resolved)
    }

    pub fn ensure_initialized(&self, clazz: ClassRef<'a>) -> VMPartialResult<()> {
        if self.vm.class_manager.expect_class_state(clazz.id, ClassLoadingState::INITIALIZED) {
            return Ok(VMResultType::Successful(()));
        }
        let to_init = self.vm.class_manager.get_classes_to_initialize(clazz)?;
        if to_init.len() > 0{
            let count = to_init.iter()
                .map(|clazz| {
                    self.vm.class_manager.update_class_state(clazz, ClassLoadingState::INITIALIZING);
                    self.init_class(clazz)
                })
                .filter(Option::is_some)
                .count();
            if count > 0{
                Ok(VMResultType::Interrupted(count, true))
            } else {
                Ok(VMResultType::Successful(()))
            }
        } else {
            Ok(VMResultType::Successful(()))
        }
    }

    fn init_class(&self, class: ClassRef<'a>) -> Option<()>{
        info!("IC[{}]", class.name);
        if class.transitive_field_count > 0 && !class.is_array(){
            let static_object = self.vm.new_object_from_class(class);
            if let Ok(mut res) = self.vm.static_class_objects.write() {
                res.insert(class.id, static_object);
            } else {
                unreachable!("Could not acquire lock for class init")
            }
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                self.thread.call_stack.create_and_push_call_frame(class_and_method, Some(static_object), Vec::new(), false);
                return Some(());
            }
        }
        self.vm.class_manager.update_class_state(class, ClassLoadingState::INITIALIZED);
        None
    }

    pub fn new_object(&self, class_name: &str) -> VMPartialResult<Reference<'a>>{
        get_or_init_special!(self.get_or_initialize_class(class_name)?, |class| Ok(VMResultType::Successful(self.vm.new_object_from_class(class))))
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
    Native(String),
    
    #[error("VM thread is poisoned: {0}")]
    LockError(String)
}

impl<T: Debug> From<PoisonError<T>> for VmError {
    fn from(value: PoisonError<T>) -> Self {
        VmError::LockError(format!("{:?}", value))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ProgramCounter(pub u16);