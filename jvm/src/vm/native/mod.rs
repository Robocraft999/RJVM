use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::class::ClassAndMethod;
use crate::vm::jni::types::{jvalue, JavaVM};
use crate::vm::native::external::ExternNativeMethod;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};
use libffi::high::CodePtr;
use libloading::Library;
use log::{info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

mod external;
mod java_lang_system;
mod java_lang_class;
mod java_lang_classloader;
mod java_lang_numbers;
mod java_lang_object;
mod sun_misc_unsafe;
mod java_lang;
mod sun_misc;
mod sun_reflect;
mod java_io;
mod misc;
mod method_handles;

macro_rules! wrap_init{
    ($macro_vm:expr, $macro_java_vm:expr, $x:expr) => {
        {
            let macro_current_frame_index: isize = $macro_vm.call_stack.len() as isize -1;
            let mut macro_counter = 0;
            let mut current_res = $x;
            while let crate::vm::VMResultType::Interrupted(..) = current_res {
                if macro_counter >= 10{
                    panic!("[wrap_init]: irschendewann is och mal schluss")
                }
                let init_res = $macro_vm.invoke_frames_until($macro_java_vm, macro_current_frame_index)?;
                if let crate::vm::VMResultType::ExceptionThrown = init_res{
                    panic!("[wrap_init]: exception thrown: {:?}", $macro_vm.caught_exception.borrow());
                }
                current_res = $x;
                macro_counter += 1;
            }
            match current_res {
                crate::vm::VMResultType::Successful(t) => t,
                other => unreachable!("[wrap_init] {:?}", other),
            }
        }
    }
}

use wrap_init;

macro_rules! gen_delegate {
    ($name:ident, |$vm:ident, $java_vm:ident, $obj:ident, $args:ident| $body:block) => {
        fn $name<'a>(
            $vm: &VM<'a>,
            $java_vm: &JavaVM,
            $obj: Option<Reference<'a>>,
            $args: Vec<Value<'a>>,
        ) -> VMPartialResult<Option<Value<'a>>> {
            $body
        }
    };
}
use gen_delegate;



pub struct NativeMethodRegistry<'a>{
    methods: Vec<NativeMethod<'a>>,
    loaded_libraries: RefCell<Vec<Library>>,
    extern_methods: RefCell<HashMap<ClassAndMethod<'a>, ExternNativeMethod>>, //FIXME consider saving native as option to prevent duplicate lookup
    exception_in_native: RefCell<bool>,
}

impl <'a> NativeMethodRegistry<'a> {
    pub fn new() -> Self{
        Self{
            methods: Vec::new(),
            loaded_libraries: RefCell::new(Vec::new()),
            extern_methods: RefCell::new(HashMap::new()),
            exception_in_native: RefCell::new(false),
        }
    }

    fn register(&mut self, class_name: &str, method_name: &str, method_descriptor: &str, delegate: NativeMethodDelegate<'a>){
        self.methods.push(NativeMethod {
            class_name: class_name.to_string(),
            method_name: method_name.to_string(),
            method_descriptor: MethodDescriptor::new(method_descriptor.to_string()),
            delegate
        })
    }

    fn add_loaded_library(&self, lib: Library){
        self.loaded_libraries.borrow_mut().push(lib);
    }

    fn try_resolve_extern_native(&self, class_and_method: &ClassAndMethod<'a>) -> bool {
        let (short, long) = class_and_method.native_escaped();
        println!("{}", class_and_method.format());
        println!("[try_resolve_extern_native]: {short} {long}");
        for lib in self.loaded_libraries.borrow().iter(){
            unsafe {
                let ptr = if let Ok(sym) = lib.get::<*const ()>(short.as_bytes()){
                    CodePtr::from_ptr(*sym as * const c_void)
                } else if let Ok(sym) = lib.get::<*const ()>(long.as_bytes()){
                    CodePtr::from_ptr(*sym as * const c_void)
                } else { continue };
                self.extern_methods.borrow_mut().insert(class_and_method.clone(), ExternNativeMethod::new(ptr, &class_and_method.method.descriptor));
                return true
            }
        }
        false
    }

    pub fn mark_exception(&self){
        warn!(target: "native", "Some native function marked as failed");
        self.exception_in_native.replace(true);
    }

    pub fn invoke(vm: &VM<'a>, java_vm: &JavaVM, cam: &ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Option<VMPartialResult<Option<Value<'a>>>>{
        for method in &vm.native_method_registry.methods{
            if method.method_name == cam.method.name && method.method_descriptor == cam.method.descriptor && cam.class.name == method.class_name{
                let needed_arg_count = cam.method.descriptor.args.len();
                let provided_arg_count = args.iter().filter(|v| v != &&Value::Dummy).count();
                info!("METHOD_NAME (custom native): {}", cam.format());
                if needed_arg_count == provided_arg_count || cam.class.has_method_polymorphic_signature(cam.method){
                    return Some((method.delegate)(vm, java_vm, object, args))
                }
                return Some(invalidation!("expected {} args but got: {}:{:?}", needed_arg_count, provided_arg_count, args))
            }
        }
        if vm.native_method_registry.try_resolve_extern_native(cam){
            let optional_extern = vm.native_method_registry.extern_methods.borrow().get(&cam).cloned();
            if let Some(extern_native) = optional_extern {
                let class_object_or_this = if cam.method.is_static(){
                    vm.try_new_class_object(cam.class).ok()?
                } else {
                    object.unwrap()
                };
                println!("[try_resolve_extern_native]: {class_object_or_this:?} with args: \n{:?}", args);
                info!("METHOD_NAME (extern native): {}", cam.format());
                let jni_result = extern_native.call(java_vm, cam, class_object_or_this, args);
                let result = if let Some(val) = jni_result{
                    unsafe {
                        Some(match (cam.method.descriptor.return_type.clone().unwrap(), val){
                            (FieldType::Primitive(PrimitiveType::Boolean), jvalue { z }) => Value::Integer(z as i32),
                            (FieldType::Primitive(PrimitiveType::Byte), jvalue { b }) => Value::Integer(b as i32),
                            (FieldType::Primitive(PrimitiveType::Char), jvalue { c }) => Value::Integer(c as i32),
                            (FieldType::Primitive(PrimitiveType::Double), jvalue { d }) => Value::Double(d as f64),
                            (FieldType::Primitive(PrimitiveType::Float), jvalue { f }) => Value::Float(f as f32),
                            (FieldType::Primitive(PrimitiveType::Integer), jvalue { i }) => Value::Integer(i as i32),
                            (FieldType::Primitive(PrimitiveType::Long), jvalue { j }) => Value::Long(j as i64),
                            (FieldType::Primitive(PrimitiveType::Short), jvalue { s }) => Value::Integer(s as i32),
                            (_, jvalue { l }) => {
                                if l == 0{
                                    vm.null()
                                } else if let Some(reference) = vm.objects_by_id.borrow().get(&(l as u32)){
                                    Value::Reference(reference)
                                } else {
                                    return Some(invalidation!("object with id {} does not exist", l))
                                }
                            }
                        })
                    }
                } else {
                    None
                };
                return if vm.native_method_registry.exception_in_native.replace(false){
                    Some(Ok(VMResultType::ExceptionThrown))
                } else {
                    Some(Ok(VMResultType::Successful(result)))
                };
            }
        }
        None
    }
}

pub struct NativeMethod<'a>{
    class_name: String,
    method_name: String,
    method_descriptor: MethodDescriptor,
    delegate: NativeMethodDelegate<'a>
}

type NativeMethodDelegate<'a> = fn(&VM<'a>, &JavaVM, Option<Reference<'a>>, Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>;

pub fn register_all_natives(registry: &mut NativeMethodRegistry) {
    java_io::register_natives(registry);
    java_lang::register_natives(registry);
    java_lang_class::register_natives(registry);
    java_lang_classloader::register_natives(registry);
    java_lang_numbers::register_natives(registry);
    java_lang_object::register_natives(registry);
    java_lang_system::register_natives(registry);
    method_handles::register_natives(registry);
    misc::register_natives(registry);
    sun_misc::register_natives(registry);
    sun_misc_unsafe::register_natives(registry);
    sun_reflect::register_natives(registry);
}

fn non_failing_some<'a>(value: Value<'a>) -> VMPartialResult<Option<Value<'a>>>{
    Ok(VMResultType::Successful(Some(value)))
}

fn non_failing_none<'a>() -> VMPartialResult<Option<Value<'a>>> {
    Ok(VMResultType::Successful(None))
}
macro_rules! invalidation {
    ($raw:expr) => { Err(VmError::ValidationError(String::from($raw))) };
    ($($arg:tt)*) => { Err(VmError::ValidationError(format!($($arg)*))) };
}
use invalidation;