use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::class::ClassAndMethod;
use crate::vm::java_thread::JavaThread;
use crate::vm::jni::types::{jvalue, JavaVM};
use crate::vm::native::external::ExternNativeMethod;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{RefId, Reference, Value};
use crate::vm::{Context, VmError, VM};
use libffi::high::CodePtr;
use libloading::Library;
use log::{info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::RwLock;

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
    ($macro_context:expr, $x:expr) => {
        {
            let macro_current_frame_index: isize = $macro_context.thread.call_stack.len() as isize -1;
            let mut current_res = $x;
            if let crate::vm::VMResultType::Interrupted(..) = current_res {
                let _ = crate::vm::java_thread::JavaThread::invoke_frames_until($macro_context, macro_current_frame_index)?;
                current_res = $x
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
    ($name:ident, |$context:ident, $obj:ident, $args:ident| $body:block) => {
        fn $name<'a>(
            $context: crate::vm::Context<'a, '_>,
            $obj: Option<Reference<'a>>,
            $args: Vec<Value>,
        ) -> VMPartialResult<Option<Value>> {
            $body
        }
    };
}
use gen_delegate;



pub struct NativeMethodRegistry<'a>{
    methods: Vec<NativeMethod<'a>>,
    loaded_libraries: RwLock<Vec<Library>>,
    extern_methods: RwLock<HashMap<ClassAndMethod<'a>, ExternNativeMethod>>, //FIXME consider saving native as option to prevent duplicate lookup
    exception_in_native: RwLock<bool>,
}

impl <'a> NativeMethodRegistry<'a> {
    pub fn new() -> Self{
        Self{
            methods: Vec::new(),
            loaded_libraries: RwLock::new(Vec::new()),
            extern_methods: RwLock::new(HashMap::new()),
            exception_in_native: RwLock::new(false),
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

    fn add_loaded_library(&self, lib: Library) {
        let Ok(mut res) = self.loaded_libraries.write() else {
            unreachable!("Could not acquire lock for loaded libs")
        };
        res.push(lib);
    }

    fn try_resolve_extern_native(&self, class_and_method: &ClassAndMethod<'a>) -> bool {
        let (short, long) = class_and_method.native_escaped();
        println!("{}", class_and_method.format());
        println!("[try_resolve_extern_native]: {short} {long}");
        if let Ok(res) = self.loaded_libraries.read() {
            for lib in res.iter(){
                unsafe {
                    let ptr = if let Ok(sym) = lib.get::<*const ()>(short.as_bytes()){
                        CodePtr::from_ptr(*sym as * const c_void)
                    } else if let Ok(sym) = lib.get::<*const ()>(long.as_bytes()){
                        CodePtr::from_ptr(*sym as * const c_void)
                    } else { continue };
                    if let Ok(mut res2) = self.extern_methods.write() {
                        res2.insert(class_and_method.clone(), ExternNativeMethod::new(ptr, &class_and_method.method.descriptor));
                    } else {
                        unreachable!("Could not acquire lock for extern methods")
                    }
                    return true
                }
            }
        }
        false
    }

    pub fn mark_exception(&self){
        warn!(target: "native", "Some native function marked as failed");
        if let Ok(mut res) = self.exception_in_native.write() {
            *res = true;
        } else {
            unreachable!("Could not acquire lock for exception in native")
        }
    }

    pub fn invoke(ctx: Context<'a, '_>, cam: &ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value>) -> Option<VMPartialResult<Option<Value>>>{
        for method in &ctx.vm.native_method_registry.methods{
            if method.method_name == cam.method.name && method.method_descriptor == cam.method.descriptor && cam.class.name == method.class_name{
                let needed_arg_count = cam.method.descriptor.args.len();
                let provided_arg_count = args.iter().filter(|v| v != &&Value::Dummy).count();
                info!("METHOD_NAME (custom native): {}", cam.format());
                if needed_arg_count == provided_arg_count || cam.class.has_method_polymorphic_signature(cam.method){
                    return Some((method.delegate)(ctx, object, args))
                }
                return Some(invalidation!("expected {} args but got: {}:{:?}", needed_arg_count, provided_arg_count, args))
            }
        }
        if ctx.vm.native_method_registry.try_resolve_extern_native(cam){
            let optional_extern = if let Ok(res) = ctx.vm.native_method_registry.extern_methods.read() {
                res.get(&cam).cloned()
            } else {
                unreachable!("Could not acquire lock for extern methods")
            };
            if let Some(extern_native) = optional_extern {
                let class_object_or_this = if cam.method.is_static(){
                    ctx.vm.try_new_class_object(cam.class).ok()?
                } else {
                    object.unwrap()
                };
                println!("[try_resolve_extern_native]: {class_object_or_this:?} with args: \n{:?}", args);
                info!("METHOD_NAME (extern native): {}", cam.format());
                let jni_result = extern_native.call(cam, class_object_or_this, args);
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
                            (_, jvalue { l }) => { Value::Reference(RefId(l as u32)) }
                        })
                    }
                } else {
                    None
                };
                if let Ok(mut res) = ctx.vm.native_method_registry.exception_in_native.write() {
                    return if *res {
                        *res = false;
                        Some(Ok(VMResultType::ExceptionThrown))
                    } else {
                        Some(Ok(VMResultType::Successful(result)))
                    };
                } else {
                    unreachable!("Could not acquire lock for exception in native")
                }
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

//FIXME maybe create a RNIEnv which is passed to the delegates instead
type NativeMethodDelegate<'a> = fn(Context<'a, '_>, Option<Reference<'a>>, Vec<Value>) -> VMPartialResult<Option<Value>>;

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

fn non_failing_some<'a>(value: Value) -> VMPartialResult<Option<Value>>{
    Ok(VMResultType::Successful(Some(value)))
}

fn non_failing_none<'a>() -> VMPartialResult<Option<Value>> {
    Ok(VMResultType::Successful(None))
}
macro_rules! invalidation {
    ($raw:expr) => { Err(VmError::ValidationError(String::from($raw))) };
    ($($arg:tt)*) => { Err(VmError::ValidationError(format!($($arg)*))) };
}
use invalidation;