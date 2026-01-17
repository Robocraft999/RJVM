use std::any::{Any, TypeId};
use crate::error::ClassParseError;
use crate::field_info::{get_class_descriptor, FieldType, PrimitiveType};
use crate::method_info::MethodDescriptor;
use crate::vm::class::{ClassAndMethod, ClassRef};
use crate::vm::jni::types::{JavaVM, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, jboolean, JNIEnv, jobject, jvalue};
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VmError, VM};
use libloading::{Library, Symbol};
use log::{debug, info, trace, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::ffi::{c_schar, c_uchar, c_ushort, c_void};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use libffi::low::CodePtr;
use libffi::middle::{Arg, Cif, Type};
use crate::access_flags::MethodFlag;
use crate::vm::java_error::JavaError;

macro_rules! wrap_init{
    ($macro_vm:expr, $macro_java_vm:expr, $x:expr) => {
        {
            let macro_current_frame_index: isize = $macro_vm.call_stack.len() as isize -1;
            let mut macro_counter = 0;
            let mut current_res = $x;
            while let VMResultType::Interrupted(..) = current_res {
                if macro_counter >= 10{
                    panic!("[wrap_init]: irschendewann is och mal schluss")
                }
                let init_res = $macro_vm.invoke_frames_until($macro_java_vm, macro_current_frame_index)?;
                if let VMResultType::ExceptionThrown = init_res{
                    panic!("[wrap_init]: exception thrown: {:?}", $macro_vm.caught_exception.borrow());
                }
                current_res = $x;
                macro_counter += 1;
            }
            match current_res {
                VMResultType::Successful(t) => t,
                other => unreachable!("[wrap_init] {:?}", other),
            }
        }
    }
}

fn primitive_type_to_native(primitive_type: &PrimitiveType) -> Type {
    match primitive_type {
        PrimitiveType::Boolean => Type::c_uchar(),
        PrimitiveType::Byte => Type::c_schar(),
        PrimitiveType::Char => Type::c_ushort(),
        PrimitiveType::Double => Type::f64(),
        PrimitiveType::Float => Type::f32(),
        PrimitiveType::Integer => Type::c_int(),
        PrimitiveType::Long => Type::c_long(),
        PrimitiveType::Short => Type::c_short(),
    }
}

fn field_type_to_native(field_type: &FieldType) -> Type{
    match field_type {
        FieldType::Object(..) => Type::pointer(),
        FieldType::Array(..) => Type::pointer(),
        FieldType::Primitive(pt) => primitive_type_to_native(pt)
    }
}

fn descriptor_to_cif(method_descriptor: &MethodDescriptor) -> Cif {
    // first arg is always JNIEnv, second arg is this / class depending on whether static
    let args: Vec<Type> = vec![Type::pointer(), primitive_type_to_native(&PrimitiveType::Integer)]
        .into_iter()
        .chain(method_descriptor.args.iter().map(field_type_to_native))
        .collect();
    let return_type = if let Some(ft) = &method_descriptor.return_type{
        field_type_to_native(ft)
    } else {
        Type::void()
    };
    Cif::new(args, return_type)
}

fn values_to_jni_args<'a>(args: &'a Vec<Value>) -> Vec<Arg<'a>> {
    args.iter().filter(|v| if let Value::Dummy = v {false} else {true}).map(|arg| {
        match arg{
            Value::Reference(reference) => Arg::new(&reference.id),
            Value::Integer(integer) => Arg::new(integer),
            Value::Long(long) => Arg::new(long),
            Value::Float(float) => Arg::new(float),
            Value::Double(double) => Arg::new(double),
            val => unreachable!("Value of type: {:?} cannot be converted to an arg", val)
        }
    }).collect()
}

pub struct NativeMethodRegistry<'a>{
    methods: Vec<NativeMethod<'a>>,
    loaded_libraries: RefCell<Vec<Library>>,
    extern_methods: RefCell<HashMap<ClassAndMethod<'a>, ExternNativeMethod>>, //FIXME consider saving native as option to prevent duplicate lookup
}

impl <'a>NativeMethodRegistry<'a>{
    pub fn new() -> Self{
        Self{
            methods: Vec::new(),
            loaded_libraries: RefCell::new(Vec::new()),
            extern_methods: RefCell::new(HashMap::new())
        }
    }

    fn register(&mut self, class_name: &str, method_name: &str, method_descriptor: &str, delegate: NativeMethodDelegate<'a>){
        self.methods.push(NativeMethod{
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
                let cif = descriptor_to_cif(&class_and_method.method.descriptor);
                self.extern_methods.borrow_mut().insert(class_and_method.clone(), ExternNativeMethod::new(ptr, cif));
                return true
            }
        }
        false
    }

    pub fn invoke(vm: &VM<'a>, java_vm: &JavaVM, class_and_method: &ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Option<VMPartialResult<Option<Value<'a>>>>{
        for method in &vm.native_method_registry.methods{
            if method.method_name == class_and_method.method.name && method.method_descriptor == class_and_method.method.descriptor && class_and_method.class.name == method.class_name{
                let needed_arg_count = class_and_method.method.descriptor.args.len();
                let provided_arg_count = args.iter().filter(|v| v != &&Value::Dummy).count();
                if needed_arg_count == provided_arg_count{
                    return Some((method.delegate)(vm, java_vm, class_and_method.class, object, args))
                }
                return Some(Err(VmError::ValidationError(format!("expected {} args but got: {}:{:?}", needed_arg_count, provided_arg_count, args))))
            }
        }
        if vm.native_method_registry.try_resolve_extern_native(class_and_method){
            let optional_extern = vm.native_method_registry.extern_methods.borrow().get(&class_and_method).cloned();
            if let Some(extern_native) = optional_extern {
                let class_object_or_this = if class_and_method.method.is_static(){
                    vm.try_new_class_object(class_and_method.class.name.as_str(), class_and_method.class.id).ok()?
                } else {
                    object.unwrap()
                };
                println!("[try_resolve_extern_native]: {class_object_or_this:?} with args: \n{:?}", args);
                let jni_result = extern_native.call(java_vm, class_and_method, class_object_or_this, args);
                let result = if let Some(val) = jni_result{
                    unsafe {
                        Some(match (class_and_method.method.descriptor.return_type.clone().unwrap(), val){
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
                                    return Some(Err(VmError::ValidationError(format!("object with id {} does not exist", l))))
                                }
                            }
                        })
                    }
                } else {
                    None
                };
                return Some(Ok(VMResultType::Successful(result)));
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

type NativeMethodDelegate<'a> = fn(&VM<'a>, &JavaVM, ClassRef<'a>, Option<Reference<'a>>, Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>;

#[derive(Debug, Clone)]
pub struct ExternNativeMethod{
    ptr: CodePtr,
    cif: Cif
}

impl ExternNativeMethod{
    pub fn new(ptr: CodePtr, cif: Cif) -> Self{
        Self { ptr, cif }
    }

    pub fn call<'a>(&self, java_vm: &JavaVM, class_and_method: &ClassAndMethod, object: Reference<'a>, args: Vec<Value<'a>>) -> Option<jvalue>{
        let env: *const JNIEnv = &java_vm.env;
        let second = object.id as jobject;
        let mut jni_args = vec![Arg::new(&env), Arg::new(&second)];
        jni_args.extend(values_to_jni_args(&args));
        unsafe {
            match class_and_method.method.descriptor.return_type{
                Some(FieldType::Object(..)) | Some(FieldType::Array(..)) => {
                    let object_id: jobject = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {l: object_id})
                }
                Some(FieldType::Primitive(PrimitiveType::Boolean)) => {
                    let val: jboolean = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {z: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Byte)) => {
                    let val: jbyte = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {b: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Char)) => {
                    let val: jchar = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {c: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Double)) => {
                    let val: jdouble = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {d: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Float)) => {
                    let val: jfloat = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {f: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Integer)) => {
                    let val: jint = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {i: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Long)) => {
                    let val: jlong = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {j: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Short)) => {
                    let val: jshort = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {s: val})
                }
                None => {
                    let _: c_void = self.cif.call(self.ptr, jni_args.as_slice());
                    None
                }
            }
        }
    }
}

pub fn register_all_natives(registry: &mut NativeMethodRegistry){
    registry.register("Test", "nop3", "()I", |_, _, _, _, _| non_failing_some(Value::Integer(-1)));
    registry.register("java/lang/System", "nanoTime", "()J", delegate_nano_time);
    registry.register("java/lang/System", "currentTimeMillis", "()J", delegate_millis_time);
    registry.register("java/lang/System", "identityHashCode", "(Ljava/lang/Object;)I", delegate_identity_hash_code);
    registry.register("java/lang/System", "setOut0", "(Ljava/io/PrintStream;)V", delegate_set_out);
    registry.register("java/lang/System", "setErr0", "(Ljava/io/PrintStream;)V", delegate_set_err);
    registry.register("java/lang/System", "arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V", delegate_arraycopy);
    registry.register("java/lang/System", "initProperties", "(Ljava/util/Properties;)Ljava/util/Properties;", delegate_init_system_props);
    registry.register("java/lang/System", "mapLibraryName", "(Ljava/lang/String;)Ljava/lang/String;", delegate_system_map_library_name);
    registry.register("java/lang/Class", "getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_get_primitive_class);
    registry.register("java/lang/Class", "getComponentType", "()Ljava/lang/Class;", delegate_get_component_type);
    registry.register("java/lang/Class", "getClassLoader0", "()Ljava/lang/ClassLoader;", delegate_get_classloader);
    registry.register("java/lang/Class", "desiredAssertionStatus0", "(Ljava/lang/Class;)Z", delegate_desired_assertion_status);
    registry.register("java/lang/Class", "getDeclaredFields0", "(Z)[Ljava/lang/reflect/Field;", delegate_get_declared_fields0);
    registry.register("java/lang/Class", "getDeclaredConstructors0", "(Z)[Ljava/lang/reflect/Constructor;", delegate_get_declared_constructors0);
    registry.register("java/lang/Class", "getDeclaredMethods0", "(Z)[Ljava/lang/reflect/Method;", delegate_get_declared_methods0);
    registry.register("java/lang/Class", "getModifiers", "()I", delegate_get_class_modifiers);
    registry.register("java/lang/Class", "getSuperclass", "()Ljava/lang/Class;", delegate_get_super_class);
    registry.register("java/lang/Class", "forName0", "(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;", delegate_for_name0);
    registry.register("java/lang/Class", "isInterface", "()Z", delegate_is_interface);
    registry.register("java/lang/Class", "isArray", "()Z", delegate_is_array);
    registry.register("java/lang/Class", "isPrimitive", "()Z", delegate_is_primitive);
    registry.register("java/lang/Class", "isAssignableFrom", "(Ljava/lang/Class;)Z", delegate_is_assignable_from);
    registry.register("java/lang/ClassLoader", "findLoadedClass0", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_loaded_class0);
    registry.register("java/lang/ClassLoader", "findBootstrapClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_bootstrap_class);
    registry.register("java/lang/ClassLoader", "findBuiltinLib", "(Ljava/lang/String;)Ljava/lang/String;", delegate_find_builtin_lib);
    registry.register("java/lang/ClassLoader$NativeLibrary", "load", "(Ljava/lang/String;Z)V", delegate_native_lib_load);
    registry.register("java/lang/Float", "floatToRawIntBits", "(F)I", delegate_float_to_raw_bits);
    registry.register("java/lang/Double", "doubleToRawLongBits", "(D)J", delegate_double_to_raw_bits);
    registry.register("java/lang/Double", "longBitsToDouble", "(J)D", delegate_long_bits_to_double);
    registry.register("java/lang/Object", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Object", "hashCode", "()I", delegate_hashcode);
    registry.register("java/lang/Object", "clone", "()Ljava/lang/Object;", delegate_clone);
    registry.register("[Ljava/lang/Object;", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Throwable", "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    registry.register("java/lang/Throwable", "getStackTraceDepth", "()I", delegate_stack_trace_depth);
    //registry.register("sun/misc/Unsafe", "registerNatives", "()V", delegate_nop);
    registry.register("sun/misc/Unsafe", "arrayBaseOffset", "(Ljava/lang/Class;)I", delegate_array_base_offset);
    registry.register("sun/misc/Unsafe", "arrayIndexScale", "(Ljava/lang/Class;)I", delegate_array_index_scale);
    registry.register("sun/misc/Unsafe", "addressSize", "()I", delegate_address_size);
    registry.register("sun/misc/Unsafe", "objectFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_object_field_offset);
    registry.register("sun/misc/Unsafe", "staticFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_static_field_offset);
    registry.register("sun/misc/Unsafe", "getObjectVolatile", "(Ljava/lang/Object;J)Ljava/lang/Object;", delegate_get_object_volatile);
    registry.register("sun/misc/Unsafe", "getIntVolatile", "(Ljava/lang/Object;J)I", delegate_get_int_volatile);
    registry.register("sun/misc/Unsafe", "staticFieldBase", "(Ljava/lang/reflect/Field;)Ljava/lang/Object;", delegate_static_field_base);
    registry.register("sun/misc/Unsafe", "compareAndSwapObject", "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z", delegate_compare_and_swap_object);
    registry.register("sun/misc/Unsafe", "compareAndSwapInt", "(Ljava/lang/Object;JII)Z", delegate_compare_and_swap_int);
    registry.register("sun/misc/Unsafe", "compareAndSwapLong", "(Ljava/lang/Object;JJJ)Z", delegate_compare_and_swap_long);
    registry.register("sun/misc/Unsafe", "allocateMemory", "(J)J", delegate_allocate_memory);
    registry.register("sun/misc/Unsafe", "putLong", "(JJ)V", delegate_put_long);
    registry.register("sun/misc/Unsafe", "getLong", "(J)J", delegate_get_long);
    registry.register("sun/misc/Unsafe", "getByte", "(J)B", delegate_get_byte);
    registry.register("sun/misc/Unsafe", "getObject", "(Ljava/lang/Object;J)Ljava/lang/Object;", delegate_get_object_volatile);
    registry.register("sun/misc/Unsafe", "putOrderedObject", "(Ljava/lang/Object;JLjava/lang/Object;)V", delegate_put_ordered_object);
    registry.register("sun/misc/Unsafe", "defineClass", "(Ljava/lang/String;[BIILjava/lang/ClassLoader;Ljava/security/ProtectionDomain;)Ljava/lang/Class;", delegate_define_class);
    registry.register("sun/misc/Unsafe", "allocateInstance", "(Ljava/lang/Class;)Ljava/lang/Object;", delegate_allocate_instance);
    registry.register("sun/reflect/Reflection", "getCallerClass", "()Ljava/lang/Class;", delegate_get_caller_class);
    registry.register("sun/reflect/Reflection", "getClassAccessFlags", "(Ljava/lang/Class;)I", delegate_get_class_access_flags);
    registry.register("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;", delegate_current_thread);
    registry.register("java/lang/Thread", "isAlive", "()Z", delegate_is_alive);
    registry.register("java/lang/Runtime", "availableProcessors", "()I", delegate_available_processors);
    registry.register("java/lang/Runtime", "freeMemory", "()J", delegate_free_memory);
    registry.register("java/lang/ProcessEnvironment", "environ", "()[[B", delegate_environ);
    registry.register("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;", delegate_get_stack_access_control_context);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/lang/String", "intern", "()Ljava/lang/String;", delegate_string_intern);
    registry.register("sun/reflect/NativeConstructorAccessorImpl", "newInstance0", "(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_new_instance0);
    registry.register("sun/reflect/NativeMethodAccessorImpl", "invoke0", "(Ljava/lang/reflect/Method;Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_invoke0);
    registry.register("java/io/FileOutputStream", "writeBytes", "([BIIZ)V", delegate_write_bytes);
    //registry.register("java/io/FileInputStream", "initIDs", "()V", delegate_nop);
    registry.register("java/io/FileInputStream", "readBytes", "([BII)I", delegate_read_bytes);
    registry.register("java/io/FileInputStream","open0", "(Ljava/lang/String;)V", delegate_open0);
    registry.register("java/io/FileSystem", "getFileSystem", "()Ljava/io/FileSystem;", delegate_get_file_system);
    registry.register("java/io/UnixFileSystem", "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register("java/io/UnixFileSystem", "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register("java/io/UnixFileSystem", "getLastModifiedTime", "(Ljava/io/File;)J", delegate_last_modified_time);
    registry.register("rjvm/io/WinFileSystem",  "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register("rjvm/io/WinFileSystem", "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register("rjvm/io/WinFileSystem", "getFinalPath0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_get_final_path0);
    registry.register("sun/nio/fs/UnixNativeDispatcher", "init", "()I", delegate_init_unix_fs_dispatcher);
    registry.register("sun/nio/fs/UnixNativeDispatcher", "getcwd", "()[B", delegate_getcwd);
    registry.register("sun/misc/VM", "initialize", "()V", delegate_init_vm);
    registry.register("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z", delegate_vm_supports_cs8);
    registry.register("sun/misc/Signal", "findSignal", "(Ljava/lang/String;)I", delegate_find_signal);
    registry.register("sun/misc/Signal", "handle0", "(IJ)J", delegate_handle0);
    registry.register("sun/misc/URLClassPath", "getLookupCacheURLs", "(Ljava/lang/ClassLoader;)[Ljava/net/URL;", delegate_lookup_cache_urls);
    registry.register("sun/misc/Perf", "createLong", "(Ljava/lang/String;IIJ)Ljava/nio/ByteBuffer;", delegate_perf_create_long)
}

fn non_failing_some<'a>(value: Value<'a>) -> VMPartialResult<Option<Value<'a>>>{
    Ok(VMResultType::Successful(Some(value)))
}

fn non_failing_none<'a>() -> VMPartialResult<Option<Value<'a>>> {
    Ok(VMResultType::Successful(None))
}

fn delegate_nop<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_none()
}

fn delegate_nano_time<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    non_failing_some(Value::Long(nanos))
}
fn delegate_millis_time<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    non_failing_some(Value::Long(millis))
}

fn delegate_identity_hash_code<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(object)) = args.get(0){
        let mut hasher = DefaultHasher::new();
        object.id.hash(&mut hasher);
        let addr = hasher.finish() as i32;
        trace!(target: "native", "HASH: {addr} {object:?}");
        non_failing_some(Value::Integer(addr))
    } else {
        Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
    }
}

fn delegate_set_out<'a>(vm: &VM<'a>, _: &JavaVM, class : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(static_object) = vm.get_static_class_object(class.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_object.set_field(1, Value::Reference(object));
            non_failing_none()
        } else {
            Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
        }
    } else {
        Err(VmError::ValidationError(format!("Couldn't find static Object of class {}", class.name)))
    }
}

fn delegate_set_err<'a>(vm: &VM<'a>, _: &JavaVM, class : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(static_object) = vm.get_static_class_object(class.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_object.set_field(2, Value::Reference(object));
            non_failing_none()
        } else {
            Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
        }
    } else {
        Err(VmError::ValidationError(format!("Couldn't find static Object of class {}", class.name)))
    }
}

fn delegate_arraycopy<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(arg0), Some(arg1), Some(arg2), Some(arg3)) = (args.get(0), args.get(1), args.get(2), args.get(3)){
        let ref1 = arg0.expect_reference()?;
        let src_pos = arg1.expect_int()? as usize;
        let ref2 = arg2.expect_reference()?;
        let dst_pos = arg3.expect_int()? as usize;
        if let (Some(arg4), ReferenceType::Array(_, _, src), ReferenceType::Array(_, _, dst)) = (args.get(4), &ref1.reference_type, &ref2.reference_type){
            let length = arg4.expect_int()? as usize;
            for i in 0..length {
                let src_index = src_pos + i;
                let dest_index = dst_pos + i;
                dst.borrow_mut()[dest_index] = src.borrow()[src_index].clone();
            }
            return non_failing_none()
        }
    }
    Err(VmError::ValidationError("Expected two arrays with indices".to_string()))
}

fn delegate_init_system_props<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let properties_object = args.get(0).unwrap().expect_reference()?;
    let mut props = vec![
        ("file.encoding", "UTF-8".to_string()),
        ("line.separator", "\n".to_string()),
        ("file.separator", "/".to_string()),
        ("path.separator", ":".to_string()),
        ("java.lang.Integer.IntegerCache.high", "127".to_string()),
        //("sun.boot.library.path", "/home/admin/.jdks/temurin-22.0.1/lib".to_string()),
        ("java.home", "/home/admin/.jdks/temurin-1.8.0_462".to_string()),
        ("sun.boot.library.path", "/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/".to_string()),
        ("sun.boot.class.path", "resources/rt.jar:resources/resources.jar".to_string()),
        ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
        ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
        ("os.name", "Linux".to_string()),
        ("os.arch", "x86_64".to_string()),
        ("java.awt.graphicsenv", "sun.awt.X11GraphicsEnvironment".to_owned())
    ];
    if env::consts::OS == "windows"{
        props = vec![
            ("file.encoding", "UTF-8".to_string()),
            ("line.separator", "\r\n".to_string()),
            ("file.separator", "\\\\".to_string()),
            ("path.separator", ";".to_string()),
            ("java.lang.Integer.IntegerCache.high", "127".to_string()),
            ("sun.boot.library.path", "C:\\Users\\Admin\\.jdks\\azul-22.0.1\\bin".to_string()),
            ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
            ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
            ("os.name", "Windows".to_string()),
        ];
    }
    let properties_set_method = vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let current_frame_index = vm.call_stack.frames.borrow().len() as isize - 1;
    for (key, value) in props.into_iter(){
        //FIXME could be bad to unwrap
        let arg1 = vm.try_new_string_object(key)?;
        let arg2 = vm.try_new_string_object(value.as_str())?;
        vm.call_stack.create_and_push_call_frame(properties_set_method.clone(), Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)], false)
    }
    let res = vm.invoke_frames_until(java_vm, current_frame_index)?;
    //Ok(VMResultType::NeedsClassInit(frames, false))
    non_failing_some(vm.null())
}

fn delegate_system_map_library_name<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(string) = args.get(0) {
        let name = VM::extract_string_from_object(string)?;
        let new_name = match env::consts::OS{
            "windows" => name + ".dll",
            "linux" => format!("lib{name}.so"),
            _ => name
        };
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(new_name.as_str())?)))
    } else {
        Err(VmError::ValidationError(format!("Expected Reference but found '{:?}'", args.get(0))))
    }
}

fn delegate_get_primitive_class<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let string = VM::extract_string_from_object(args.get(0).unwrap())?;
    match string.as_str() {
        "int"     => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(  "java/lang/Integer")?))),
        "long"    => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(     "java/lang/Long")?))),
        "short"   => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(    "java/lang/Short")?))),
        "char"    => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name("java/lang/Character")?))),
        "byte"    => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(     "java/lang/Byte")?))),
        "float"   => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(    "java/lang/Float")?))),
        "double"  => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(   "java/lang/Double")?))),
        "boolean" => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(  "java/lang/Boolean")?))),
        "void"    => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(     "java/lang/Void")?))),
        _ => Err(VmError::ValidationError(format!("Expected extractable string")))
    }
}

fn delegate_get_component_type<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, class_object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("getComponentType \n'{:?}'\n'{:?}'", class_object, args);
    let class_name = VM::extract_class_name_from_class_object(class_object.unwrap())?;
    //let field_type = field_type_from_str(class_name.as_str());
    debug!("getComponentType '{:?}'", class_name);

    let array_class = vm.get_or_resolve_class(class_name.as_str())?;
    if let Some(array_info) = &array_class.array_info{
        let component_class_object = wrap_init!(vm, java_vm, vm.new_class_object_by_name(array_info.component_type.to_class_name().as_str())?);
        non_failing_some(Value::Reference(component_class_object))
    } else {
        Err(VmError::ValidationError(format!("Expected Array object but found '{:?}'", class_object)))
    }
}

fn delegate_get_classloader<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //TODO check
    debug!("getClassLoader0");
    non_failing_some(vm.null())
}

fn delegate_desired_assertion_status<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //TODO check
    debug!("desiredAssertionStatus0");
    non_failing_some(Value::Integer(1))
}

fn delegate_get_declared_fields0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("getDeclaredFields");
    if let Some(clazz) = class_object {
        let class_name = VM::extract_class_name_from_class_object(clazz)?;
        debug!("class name: {}", class_name);
        let class_ref = vm.get_or_resolve_class(class_name.as_str())?;
        let mut content = Vec::new();
        for field in class_ref.fields.iter(){
            let java_field = wrap_init!(vm, java_vm, vm.new_object("java/lang/reflect/Field")?);
            //name
            java_field.set_field(6, Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(field.name.as_str())?)));
            //clazz
            java_field.set_field(4, Value::Reference(clazz));
            //modifiers
            java_field.set_field(8, Value::Integer(field.flags.iter().cloned().map(|flag| flag as u16 as i32).reduce(|flag1, flag2| flag1 | flag2).unwrap_or(0)));
            //type
            let type_class_object = wrap_init!(vm, java_vm, vm.new_class_object_by_name(field.field_type.to_class_name().as_str())?);
            java_field.set_field(7, Value::Reference(type_class_object));
            info!("field name: {}", field.name);
            content.push(Value::Reference(java_field));
        }
        for field in content.iter(){
            if let Value::Reference(java_field) = field {
                debug!("field : {:?}", java_field);
                if let ReferenceType::Object(fields) = &java_field.reference_type{
                    for field_field in fields.borrow().iter(){
                        debug!("field_: {:?}", field_field);
                    }
                }
            }
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Field".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        //FIXME i dont know if this should be none
        non_failing_none()
    }
}

fn delegate_get_declared_constructors0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, class_object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("getDeclaredConstructors0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_object, args.get(0)){
        let class = vm.extract_class_from_class_object(class_ref)?;
        let java_constructor_class = wrap_init!(vm, java_vm, vm.get_or_initialize_class("java/lang/reflect/Constructor")?);
        let mut content = Vec::new();
        for constructor in class.get_constructors(*public_only == 1).iter(){
            let java_constructor = vm.new_object_from_class(java_constructor_class);

            //clazz
            java_constructor.set_field(7, Value::Reference(class_ref));

            let mut parameters = Vec::new();
            for field_type in constructor.descriptor.args.iter(){
                let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_by_name(field_type.to_class_name().as_str())?);
                parameters.push(Value::Reference(parameter_class));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = constructor.exceptions.clone(){
                for exception in exception_vec.0{
                    let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_by_name(exception.as_str())?);
                    exceptions.push(Value::Reference(parameter_class));
                }
            }
            //parameterTypes
            java_constructor.set_field(9, Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(parameters.clone()))?)));
            
            //exceptionTypes
            java_constructor.set_field(10, Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(exceptions.clone()))?)));

            let flags = constructor.flags.iter().map(|flag| flag.clone() as u16).fold(0, |flag1, flag2| flag1 | flag2);
            //modifiers
            java_constructor.set_field(11, Value::Integer(flags as i32));

            content.push(Value::Reference(java_constructor));
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Constructor".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        Err(VmError::ValidationError("Expected Class object and boolean".to_string()))
    }
}

fn delegate_get_declared_methods0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, class_object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>> {
    debug!("getDeclaredMethods0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_object, args.get(0)){
        let class = vm.extract_class_from_class_object(class_ref)?;
        let mut content = Vec::new();
        for method in class.get_methods(*public_only == 1).iter(){
            let java_method = wrap_init!(vm, java_vm, vm.new_object("java/lang/reflect/Method")?);

            //clazz
            java_method.set_field(7, Value::Reference(class_ref));

            let name = wrap_init!(vm, java_vm, vm.new_string_object(&method.name.as_str())?);
            //name
            java_method.set_field(9, Value::Reference(name));

            let return_type = if let Some(f) = &method.descriptor.return_type{
                Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(f.to_class_name().as_str())?))
            } else {
                vm.null()
            };
            let mut parameters = Vec::new();
            for field_type in method.descriptor.args.iter(){
                let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_by_name(field_type.to_class_name().as_str())?);
                parameters.push(Value::Reference(parameter_class));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = method.exceptions.clone(){
                for exception in exception_vec.0{
                    let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_by_name(exception.as_str())?);
                    exceptions.push(Value::Reference(parameter_class));
                }
            }

            //returnType
            java_method.set_field(10, return_type);

            //parameterTypes
            java_method.set_field(11, Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(parameters.clone()))?)));

            //exceptionTypes
            java_method.set_field(12, Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(exceptions.clone()))?)));

            let flags = method.flags.iter().map(|flag| flag.clone() as u16).fold(0, |flag1, flag2| flag1 | flag2);
            //modifiers
            java_method.set_field(13, Value::Integer(flags as i32));

            content.push(Value::Reference(java_method));
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Method".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        Err(VmError::ValidationError("Expected Class object and boolean".to_string()))
    }
}

fn delegate_get_class_modifiers<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(obj) = class_object{
        let class = vm.extract_class_from_class_object(obj)?;
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_get_super_class<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, this: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(obj) = this {
        let class = vm.extract_class_from_class_object(obj)?;
        match class.superclass {
            Some(super_class) => {
                let super_class_object = wrap_init!(vm, java_vm, vm.new_class_object_by_name(super_class.name.as_str())?);
                non_failing_some(Value::Reference(super_class_object))
            }
            None => non_failing_some(vm.null())
        }

    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_for_name0<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("forName0");
    let exception = |name: &str| {
        let exception_class_name = String::from("java/lang/ClassNotFoundException");
        let exception_message = format!("Class '{}' was not found", name);

        let exception_class = wrap_init!(vm, java_vm, vm.get_or_initialize_class(&exception_class_name)?);
        let exception_object = vm.try_new_object(&exception_class_name)?;

        let details = wrap_init!(vm, java_vm, vm.new_string_object(exception_message.as_str())?);
        //detailsMessage
        exception_object.set_field(2, Value::Reference(details));

        let prev = vm.caught_exception.replace(
            Some((
                exception_message,
                String::from("java/lang/Class.forName0(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;"),
                Value::Reference(exception_object)
            )));
        assert!(prev.is_none());
        Ok(VMResultType::ExceptionThrown)
    };

    if let Some(name) = args.get(0) && !name.is_null(){
        let name = VM::extract_string_from_object(&name)?;
        let name = name.replace(".", "/");
        let current_frame_index = vm.call_stack.len() as isize -1;
        match vm.get_or_resolve_class(&name){
            Ok(..) => {
                non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(&name)?)))
            },
            Err(VmError::ParseError(ClassParseError::ResolveError(_))) => {
                exception(name.as_str())
            }
            Err(err) => Err(err)
        }
    } else {
        exception("")
    }
}

fn delegate_is_interface<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("isInterface {:?}", obj);
    if let Some(obj) = obj {
        let class = vm.extract_class_from_class_object(obj)?;
        non_failing_some(Value::from(class.is_interface()))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_array<'a>(vm: &VM<'a>, _: &JavaVM,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("isArray {:?}", obj);
    if let Some(obj) = obj {
        non_failing_some(Value::from(obj.is_array()))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_primitive<'a>(vm: &VM<'a>, _: &JavaVM,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("isPrimitive {:?}", obj);
    if let Some(obj) = obj {
        let name = VM::extract_class_name_from_class_object(obj)?;
        non_failing_some(Value::Integer(match name.as_str() {
            "java/lang/Boolean" | "java/lang/Character" | "java/lang/Byte"  | "java/lang/Short"  |
            "java/lang/Integer" | "java/lang/Long"      | "java/lang/Float" | "java/lang/Double" |
            "java/lang/Void" => 1,
            _ => 0,
        }))
        //Ok(Some(Value::Integer(if PrimitiveType::from_str(name.as_str()).is_ok() { 1 } else { 0 })))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_assignable_from<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, obj: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("isAssignableFrom\nthis: {:?}\nfrom: {:?}", obj, args);
    if let (Some(object), Some(Value::Reference(other))) = (obj, args.get(0)) {
        let this_class = vm.extract_class_from_class_object(object)?;
        let from_class = vm.extract_class_from_class_object(other)?;
        non_failing_some(Value::from(vm.unchecked_check_if_subclass_of(this_class.name.as_str(), from_class.name.as_str())?))
    } else {
        Err(VmError::ValidationError("expected a class reference".to_string()))
    }
}

fn delegate_find_loaded_class0<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("findLoadedClass0 {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = VM::extract_string_from_object(&str_object)?;
        if vm.class_manager.find_class_by_name(class_name.as_str()).is_some() {
            non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(class_name.as_str())?)))
        } else {
            non_failing_some(vm.null())
        }
    } else {
        Err(VmError::ValidationError("expected a string reference".to_string()))
    }
}

fn delegate_find_bootstrap_class<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("findBootstrapClass {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = VM::extract_string_from_object(&str_object)?;
        if vm.class_manager.find_class_by_name(class_name.as_str()).is_some() {
            non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(class_name.as_str())?)))
        } else {
            non_failing_some(vm.null())
        }
    } else {
        Err(VmError::ValidationError("expected a string reference".to_string()))
    }
}

fn delegate_find_builtin_lib<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("findBuiltinLib {:?}", args);
    //FIXME here we have to check if the library with the given name is builtin -> exports the function JNI_OnLoad_<libname>
    non_failing_some(vm.null())
}

fn delegate_native_lib_load<'a>(vm: &VM<'a>, java_vm: &JavaVM,  _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("nativeLib::load {:?}", object);
    if let Some(obj) = object {
        //handle
        obj.set_field(0, Value::Long(1));
        let name_field = obj.get_field(3);//args.get(0).unwrap();
        let name = VM::extract_string_from_object(&name_field)?;
        println!("name: {name}");
        println!("javavm: {:p}", java_vm);

        unsafe {
            use libffi::middle::{Arg, Cif, Type};
            use std::{ffi::c_void, ptr};
            //let lib = Library::new("/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/libjava.so").unwrap();
            let lib = Library::new(name).unwrap();
            let sym: Symbol<*const ()> = lib.get(b"JNI_OnLoad").unwrap();

            let func_ptr = *sym as * const c_void;
            vm.native_method_registry.add_loaded_library(lib);

            let vm_ptr = ptr::from_ref(java_vm) as *const c_void;
            println!("javavmp: {:p}", vm_ptr);
            let reserved = std::ptr::null() as *const c_void;
            let cif = Cif::new(vec![Type::pointer(), Type::pointer()], Type::i32()); //JNI_OnLoad
            let res: i32 = cif.call(libffi::low::CodePtr::from_ptr(func_ptr), &[Arg::new(&vm_ptr), Arg::new(&reserved)]);
            println!("res: {:x}", res);
        }
        obj.set_field(5, Value::from(true));

        non_failing_none()
    } else {
        Err(VmError::ValidationError("this is null".to_string()))
    }
}

fn delegate_float_to_raw_bits<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Float(value)) = args.get(0){
        return non_failing_some(Value::Integer(value.to_bits() as i32))
    }
    Err(VmError::ValidationError(format!("Expected float")))
}

fn delegate_double_to_raw_bits<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Double(value)) = args.get(0){
        return non_failing_some(Value::Long(value.to_bits() as i64))
    }
    Err(VmError::ValidationError(format!("Expected double")))
}

fn delegate_long_bits_to_double<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Long(value)) = args.get(0){
        return non_failing_some(Value::Double(f64::from_bits(*value as u64)))
    }
    Err(VmError::ValidationError(format!("Expected long")))
}

fn delegate_get_class<'a>(vm: &VM<'a>, java_vm: &JavaVM, class: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //TODO check
    debug!("getClass");
    if let Some(obj) = object {
        debug!("{} obj: {:?}", class.name, obj.class_name);
        let class_object = wrap_init!(vm, java_vm, vm.new_class_object_by_name(obj.class_name.as_str())?);
        non_failing_some(Value::Reference(class_object))
    } else {
        Err(VmError::ValidationError("Object is Null".to_string()))
    }
}

fn delegate_hashcode<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, reference: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(obj) = reference{
        let mut hasher = DefaultHasher::new();
        obj.id.hash(&mut hasher);
        let addr = hasher.finish() as i32;
        trace!(target: "native", "HASHCODE: {addr} {obj:?}");
        non_failing_some(Value::Integer(addr))
    } else {
        Err(VmError::ValidationError("Expected object".to_string()))
    }
}

fn delegate_clone<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, reference: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("clone");
    if let Some(obj) = reference{
        if obj.is_array(){
            if let ReferenceType::Array(dims, component_type, content) = &obj.reference_type{
                debug!("Cloning array: {:?}", reference);
                let new_array = Value::Reference(wrap_init!(vm, java_vm, vm.new_array(*dims, component_type.clone().to_array_field_type(*dims), content.clone())?));
                non_failing_some(new_array)
            } else {
                Err(VmError::ValidationError("Expected array to be cloned".to_string()))
            }
        } else {
            if let ReferenceType::Object(content) = &obj.reference_type{
                debug!("Cloning object: {:?}", reference);
                let mut new_object = wrap_init!(vm, java_vm, vm.new_object(obj.class_name.as_str())?);
                if let ReferenceType::Object(new_content) = &new_object.reference_type{
                    for (index, item) in content.borrow().iter().enumerate(){
                        new_content.borrow_mut().insert(index, item.clone());
                    }
                }
                non_failing_some(Value::Reference(new_object))
            } else {
                Err(VmError::ValidationError("Expected array to be cloned".to_string()))
            }
        }
    } else {
        Err(VmError::ValidationError("Expected object".to_string()))
    }
}

fn delegate_fill_in_stacktrace<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(receiver) = object{
        return non_failing_some(Value::Reference(receiver));
    }
    Err(VmError::ValidationError("Expected a Throwable".to_string()))
}

fn delegate_stack_trace_depth<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(obj) = object{
        non_failing_some(Value::Integer(0))
    } else {
        Err(VmError::ValidationError("Expected a Throwable".to_string()))
    }
}

const ARRAY_BASE_OFFSET: usize = 16;

fn delegate_array_base_offset<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(class)) = args.get(0){
        non_failing_some(Value::Integer(ARRAY_BASE_OFFSET as i32))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_array_index_scale<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(class)) = args.get(0){
        non_failing_some(Value::Integer(1))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_address_size<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Integer(8))
}

fn delegate_object_field_offset<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //FIXME calc real offset
    debug!("delegate_object_field_offset: '{:?}'", args);
    if let Some(field) = args.get(0){
        let field_ref = field.expect_reference()?;
        let clazz = field_ref.get_field(4).expect_reference()?;
        let class_ref = vm.extract_class_from_class_object(clazz)?;
        let name_val = field_ref.get_field(6);
        let name = VM::extract_string_from_object(&name_val)?;
        if let Some((index, _)) = class_ref.find_field(name.as_str()){
            non_failing_some(Value::Long(index as i64))
        } else {
            Err(VmError::ValidationError(format!("Field with name: '{}' does not exist", name)))
        }
    } else {
        Err(VmError::ValidationError("Expected an Object field reference".to_string()))
    }
}

fn delegate_static_field_offset<'a>(vm: &VM<'a>, java_vm: &JavaVM, class : ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //non_failing_some(Value::Long(0))
    //TODO check if needed
    delegate_object_field_offset(vm, java_vm, class, object, args)
}

fn delegate_get_object_volatile<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("get_object_volatile args: {:?}", args);
    if let (Some(Value::Reference(o)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize  - ARRAY_BASE_OFFSET));
        }
        let field_value = if o.class_name == "java/lang/Class"{
            let class_ref = vm.extract_class_from_class_object(o)?;
            let static_object = vm.static_class_objects.borrow().get(&class_ref.id).unwrap().clone();
            static_object.get_field(*index as usize)
        } else {
            o.get_field(*index as usize)
        };
        non_failing_some(field_value)
    } else {
        Err(VmError::ValidationError(format!("Expected an Reference or Array but got: {:?}", args)))
    }
}

fn delegate_get_int_volatile<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("get_int_volatile args: {:?}", args);
    if let (Some(Value::Reference(o)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize  - ARRAY_BASE_OFFSET));
        }
        let field_value = if o.class_name == "java/lang/Class"{
            let class_ref = vm.extract_class_from_class_object(o)?;
            let static_object = vm.static_class_objects.borrow().get(&class_ref.id).unwrap().clone();
            static_object.get_field(*index as usize)
        } else {
            o.get_field(*index as usize)
        };
        non_failing_some(field_value)
    } else {
        Err(VmError::ValidationError(format!("Expected an Reference or Array but got: {:?}", args)))
    }
}

fn delegate_static_field_base<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(field_object_value) = args.get(0){
        let field_object = field_object_value.expect_reference()?;
        trace!("staticFieldBase: on field: '{:?}'", field_object);
        let class_object = field_object.get_field(4);
        non_failing_some(class_object)
    } else {
        Err(VmError::ValidationError("Expected a field reference".to_string()))
    }
}

fn delegate_compare_and_swap_object<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(Value::Reference(o)), Some(Value::Long(offset)), Some(Value::Reference(expected)), Some(Value::Reference(x))) = (args.get(0), args.get(1), args.get(3), args.get(4)) {
        if o.is_null(){
            return Err(VmError::ValidationError("Expected an object or array but found null".to_string()))
        } else if o.is_object(){
            if let Value::Reference(current) = o.get_field(*offset as usize){
                if current.id == expected.id{
                    o.set_field(*offset as usize, Value::Reference(*x));
                    return non_failing_some(Value::from(true));
                }
            }
        } else if o.is_array(){
            if let Value::Reference(current) = o.get_element(*offset as usize - ARRAY_BASE_OFFSET){
                if current.id == expected.id{
                    o.set_element(*offset as usize - ARRAY_BASE_OFFSET, Value::Reference(*x));
                    return non_failing_some(Value::from(true));
                }
            }
        }
    }
    non_failing_some(Value::from(false))
}

fn delegate_compare_and_swap_int<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(Value::Reference(o)), Some(Value::Long(offset)), Some(Value::Integer(expected)), Some(Value::Integer(x))) = (args.get(0), args.get(1), args.get(3), args.get(4)) {
        if let Value::Integer(current) = o.get_field(*offset as usize){
            if current == *expected{
                o.set_field(*offset as usize, Value::Integer(*x));
                return non_failing_some(Value::from(true));
            }
        }
    }
    non_failing_some(Value::from(false))
}

fn delegate_compare_and_swap_long<'a>(_: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(Value::Reference(o)), Some(Value::Long(offset)), Some(Value::Long(expected)), Some(Value::Long(x))) = (args.get(0), args.get(1), args.get(3), args.get(5)) {
        if let Value::Long(current) = o.get_field(*offset as usize){
            if current == *expected{
                o.set_field(*offset as usize, Value::Long(*x));
                return non_failing_some(Value::from(true));
            }
        }
    }
    non_failing_some(Value::from(false))
}

fn delegate_allocate_memory<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Long(num)) = args.get(0){
        //return is address in memory
        let ptr = vm.unsafe_allocator.allocate_memory(*num as usize);
        non_failing_some(Value::Long(ptr))
    } else {
        Err(VmError::ValidationError("Expected a long".to_string()))
    }
}

fn delegate_put_long<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //because args = [Long, Dummy, Long, Dummy]
    if let (Some(Value::Long(ptr)), Some(Value::Long(value))) = (args.get(0), args.get(2)){
        vm.unsafe_allocator.put_long(*ptr, *value);
        non_failing_none()
    } else {
        Err(VmError::ValidationError("Expected a long as address and a long as value".to_string()))
    }
}

fn delegate_get_long<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Long(ptr)) = args.get(0){
        let long = vm.unsafe_allocator.get_long(*ptr);
        Ok(VMResultType::Successful(long.map(|val| Value::Long(val))))
    } else {
        Err(VmError::ValidationError("Expected a long as address".to_string()))
    }
}

fn delegate_get_byte<'a>(vm: &VM<'a>, _: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Long(ptr)) = args.get(0){
        let byte = vm.unsafe_allocator.get_byte(*ptr);
        Ok(VMResultType::Successful(byte.map(|byte| Value::Integer(byte as i32))))
    } else {
        Err(VmError::ValidationError("Expected a long as address".to_string()))
    }
}

fn delegate_put_ordered_object<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("put_ordered_object args: {:?}", args);
    if let (Some(Value::Reference(o)), Some(Value::Long(index)), Some(x)) = (args.get(0), args.get(1), args.get(3)) {
        if o.is_array(){
            o.set_element(*index as usize  - 16, x.clone());
            return non_failing_none();
        }
        if o.class_name == "java/lang/Class"{
            let class_ref = vm.extract_class_from_class_object(o)?;
            let _ = wrap_init!(vm, java_vm, vm.get_or_initialize_class(class_ref.name.as_str())?);
            let static_object = vm.static_class_objects.borrow().get(&class_ref.id).unwrap().clone();
            static_object.set_field(*index as usize, x.clone());
        } else {
            o.set_field(*index as usize, x.clone());
        }
        non_failing_none()
    } else {
        Err(VmError::ValidationError(format!("Expected a reference or array but got: {:?}", args)))
    }
}

fn delegate_define_class<'a>(vm: &VM<'a>, java_vm: &JavaVM, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(class_name_value), Some(Value::Reference(bytes_value)), Some(start), Some(end)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let class_name = VM::extract_string_from_object(class_name_value)?;
        let bytes = if let ReferenceType::Array(_, _, data) = &bytes_value.reference_type{
            data.borrow().iter().map(|val| if let Value::Integer(byte) = val {*byte as u8} else {0}).collect()
        } else {
            Vec::new()
        };
        let (start, end) = (start.expect_int()?, end.expect_int()?);
        let bytes = bytes.into_iter().skip(start as usize).take((end - start) as usize).collect::<Vec<_>>();
        let class_object = wrap_init!(vm, java_vm, vm.define_class(class_name.as_str(), bytes.clone())?);
        non_failing_some(Value::Reference(class_object))
    } else {
        Err(VmError::ValidationError(format!("define_class: expected string_object, byte array, start and end ints but got: {:?}, {:?}, {:?}, {:?}", args.get(0), args.get(1), args.get(2), args.get(3))))
    }
}

fn delegate_allocate_instance<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(class_object)) = args.get(0){
        let class_name = VM::extract_class_name_from_class_object(class_object)?;
        let object = wrap_init!(vm, java_vm, vm.new_object(class_name.as_str())?);
        non_failing_some(Value::Reference(object))
    } else {
        Err(VmError::ValidationError(format!("Expected a class reference to allocate but got: {:?}", args)))
    }
}

fn delegate_get_caller_class<'a>(vm: &VM<'a>, java_vm: &JavaVM, class : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let frame_index = vm.call_stack.frames.borrow().len() - 2 - 1;
    if let Some(frame) = vm.call_stack.frames.borrow().get(frame_index){
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(frame.class_and_method.class.name.as_str())?)))
    } else {
        Err(VmError::ValidationError("There is no parent Callframe".to_string()))
    }
}

fn delegate_get_class_access_flags<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(obj)) = args.get(0){
        let class = vm.extract_class_from_class_object(obj)?;
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_current_thread<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if vm.current_thread.borrow().is_none(){
        let thread = wrap_init!(vm, java_vm, vm.new_object("java/lang/Thread")?);
        //let thread_init = vm.resolve_class_method("java/lang/Thread", "<init>", "()V")?;
        //vm.invoke(thread_init, Some(thread), vec![])?;
        let name_string = wrap_init!(vm, java_vm, vm.new_string_object("Main")?);
        let name_char_array = name_string.get_field(0);

        let group_name = wrap_init!(vm, java_vm, vm.new_string_object("system")?);
        let group = wrap_init!(vm, java_vm, vm.new_object("java/lang/ThreadGroup")?);
        group.set_field(6, Value::Integer(0));
        group.set_field(1, Value::Reference(group_name));
        group.set_field(2, Value::Integer(10));
        group.set_field(0, vm.null());

        //let group_init = vm.try_resolve_class_method("java/lang/ThreadGroup", "<init>", "()V")?;
        //vm.invoke_new_frame(group_init, Some(group), vec![])?;

        thread.set_field(0, name_char_array);
        thread.set_field(1, Value::Integer(10));
        thread.set_field(8, Value::Reference(group));
        vm.current_thread.replace(Some(thread));
        non_failing_some(Value::Reference(thread))
    } else {
        non_failing_some(Value::Reference(vm.current_thread.borrow().unwrap()))
    }
}

fn delegate_is_alive<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(object.unwrap().get_field(5))
}

fn delegate_available_processors<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Integer(1))
}

fn delegate_free_memory<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Long(1024 * 1024 * 20))
}

fn delegate_environ<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let vars = vec![
        ("DISPLAY", ":0")
    ];
    fn byte_array_from_str<'s>(vm: &VM<'s>, string: &str) -> VMResult<Reference<'s>>{
        vm.try_new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(string.as_bytes().iter().map(|c| Value::Integer(*c as i32)).collect()))
    }
    let _ = wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(Vec::new()))?);
    let values: Vec<Value> = vars.iter()
        .flat_map(|(k, v)| vec![
            Value::Reference(byte_array_from_str(vm, k).unwrap()),
            Value::Reference(byte_array_from_str(vm, v).unwrap()),
        ])
        .collect();
    let array_ref = wrap_init!(vm, java_vm, vm.new_array(2, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(2), RefCell::new(values.clone()))?);
    non_failing_some(Value::Reference(array_ref))
}

fn delegate_get_stack_access_control_context<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(vm.null())
}

fn delegate_do_privileged<'a>(vm: &VM<'a>, java_vm: &JavaVM, class: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(action)) = args.get(0){
        let class_name = vm.find_class_by_id(action.class_id).unwrap().name.as_str();
        let run = vm.resolve_class_method(class_name, "run", "()Ljava/lang/Object;")?;
        let current_frame_index = vm.call_stack.len() as isize - 1;
        vm.call_stack.create_and_push_call_frame(run, Some(action), vec![], false);
        let res = vm.invoke_frames_until(java_vm, current_frame_index);

        // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
        // because it doesn't know whether it is a subroutine or not
        match res{
            Ok(any) => Ok(any),
            Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
            Err(e) => Err(e),
        }
    } else {
        Err(VmError::ValidationError("Expected a action object reference".to_string()))
    }
}

fn delegate_string_intern<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(obj) = object{
        let content = VM::extract_string_from_object(&Value::Reference(obj))?;
        if vm.string_objects.borrow().contains_key(&content){
            non_failing_some(Value::Reference(vm.string_objects.borrow()[&content]))
        } else {
            non_failing_some(Value::Reference(obj))
        }
    } else {
        Err(VmError::ValidationError("Expected a string object reference".to_owned()))
    }
}

fn delegate_new_instance0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("newInstance0");
    debug!("{:?}", args);
    if let Some(Value::Reference(constructor)) = args.get(0){
        //clazz
        let clazz = constructor.get_field(7);
        //parameterTypes
        let parameter_types = constructor.get_field(9);
        if let (Value::Reference(class_ref), Value::Reference(parameter_array)) = (clazz, parameter_types){
            if let ReferenceType::Array(_, _, type_content) = &parameter_array.reference_type{
                let class = vm.extract_class_from_class_object(class_ref)?;
                let mut descriptor = String::from("(");
                for constructor_parameter_type in type_content.borrow().iter(){
                    if let Value::Reference(parameter_type_ref) = constructor_parameter_type {
                        let class = vm.extract_class_from_class_object(parameter_type_ref)?;
                        if !class.is_array(){
                            descriptor.push_str(&get_class_descriptor(&class.name));
                        } else {
                            descriptor.push_str(&class.name);
                        }
                    }
                }
                descriptor.push_str(")V");
                if let Some(method) = class.find_method("<init>", descriptor.as_str()) {
                    debug!("method: {:?}", method);
                    let class_and_method = ClassAndMethod {class, method};
                    let constructor_args = if let Some(Value::Reference(argument_array)) = args.get(1){
                        if let ReferenceType::Array(_, _, args_content) = &argument_array.reference_type{
                            args_content.borrow().clone()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                    let object = wrap_init!(vm, java_vm, vm.new_object(class_and_method.class.name.as_str())?);
                    let current_frame_index = vm.call_stack.len() as isize - 1;
                    vm.call_stack.create_and_push_call_frame(class_and_method, Some(object), constructor_args, false);
                    let res = vm.invoke_frames_until(java_vm, current_frame_index);
                    // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
                    // because it doesn't know whether it is a subroutine or not
                    return match res { 
                        Ok(VMResultType::Successful(None)) => { non_failing_some(Value::Reference(object)) }
                        Ok(VMResultType::Successful(Some(value))) => { Err(VmError::ValidationError(format!("Constructor should not return anything: {:?}", value))) }
                        Ok(typ) => unreachable!("{:?} can't escape invoke_frames_until", typ),
                        Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
                        Err(e) => Err(e),
                    }
                }
            }
        }
        unreachable!()
    } else {
        Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
    }
}

fn delegate_invoke0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("invoke0");
    debug!("{:?}", args);
    if let (Some(Value::Reference(method)), Some(Value::Reference(obj))) = (args.get(0), args.get(1)) {
        let clazz = method.get_field(4);
        let method_name_val = method.get_field(6);
        let return_type_val = method.get_field(7);
        let parameter_types = method.get_field(8);
        if let (Value::Reference(class_ref), Value::Reference(return_type_ref), Value::Reference(parameter_array)) = (clazz, return_type_val, parameter_types) {
            if let ReferenceType::Array(_, _, type_content) = &parameter_array.reference_type {
                let class = vm.extract_class_from_class_object(class_ref)?;
                let mut descriptor = String::from("(");
                for constructor_parameter_type in type_content.borrow().iter(){
                    if let Value::Reference(parameter_type_ref) = constructor_parameter_type {
                        let class = vm.extract_class_from_class_object(parameter_type_ref)?;
                        if !class.is_array(){
                            descriptor.push_str(&get_class_descriptor(&class.name));
                        } else {
                            descriptor.push_str(&class.name);
                        }
                    }
                }
                descriptor.push_str(")");
                if !return_type_ref.is_null(){
                    let return_type = vm.extract_class_from_class_object(return_type_ref)?;
                    if !return_type.is_array(){
                        descriptor.push_str(&get_class_descriptor(&return_type.name));
                    } else {
                        descriptor.push_str(&return_type.name);
                    }
                }
                let method_name = VM::extract_string_from_object(&method_name_val)?;
                if let Some(method) = class.find_method(method_name.as_str(), descriptor.as_str()) {
                    debug!("method: {:?}", method);
                    let class_and_method = ClassAndMethod {class, method};
                    let method_args = if let Some(Value::Reference(argument_array)) = args.get(1){
                        if let ReferenceType::Array(_, _, args_content) = &argument_array.reference_type{
                            args_content.borrow().clone()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                    let current_frame_index = vm.call_stack.len() as isize - 1;
                    vm.call_stack.create_and_push_call_frame(class_and_method, if !obj.is_null() {Some(obj)} else {None}, method_args, false);
                    let res = vm.invoke_frames_until(java_vm, current_frame_index);
                    // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
                    // because it doesn't know whether it is a subroutine or not
                    return match res {
                        Ok(VMResultType::Successful(None)) => {
                            assert!(return_type_ref.is_null());
                            non_failing_some(vm.null())
                        }
                        Ok(VMResultType::Successful(Some(value))) => {
                            assert!(!return_type_ref.is_null());
                            non_failing_some(value)
                        }
                        Ok(typ) => unreachable!("{:?} can't escape invoke_frames_until", typ),
                        //FIXME return InvocationTargetException
                        Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
                        Err(e) => Err(e),
                    }
                }
            }
        }
    }
    Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
}

fn delegate_write_bytes<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(Value::Reference(bytes_ref)), Some(Value::Integer(offset)), Some(Value::Integer(amount)), Some(Value::Integer(should_append))) =
        (args.get(0), args.get(1), args.get(2), args.get(3))
    {
        if let ReferenceType::Array(_, _, data) = &bytes_ref.reference_type{
            let data = &data.borrow()[*offset as usize..(*offset + *amount) as usize];
            let string: String = data.iter().map(|value| if let Value::Integer(int) = value { (*int as u8) as char} else { '?' }).collect();
            print!("{}", string);
            non_failing_none()
        } else {
            Err(VmError::ValidationError("Expected a byte array as first arg".to_string()))
        }
    } else {
        Err(VmError::ValidationError("Expected a byte array, offset, amount and boolean".to_string()))
    }
}

//obsolete because libjava.so is loaded
fn delegate_open0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(Value::Reference(path_ref))) = (args.get(0)) && !path_ref.is_null(){
        let path = VM::extract_string_from_object(&Value::Reference(path_ref))?;
        if !vm.currently_open_files.borrow().contains_key(&path) {
            let file_content = vm.class_manager.class_path.resolve_file(path.as_str())?;
            if let Some(file_content) = file_content {
                vm.currently_open_files.borrow_mut().insert(path.clone(), (file_content, 0));
            }
        }
        non_failing_none()
    } else {
        Err(VmError::ValidationError(format!("Expected a string for the path but got: {:?}", args.get(0))))
    }

}

fn delegate_read_bytes<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, obj: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let (Some(arg0), Some(arg1), Some(arg2)) = (args.get(0), args.get(1), args.get(2)) {
        let data = arg0.expect_reference()?;
        let offset = arg1.expect_int()?;
        let length = arg2.expect_int()?;
        
        let io_exception_class = wrap_init!(vm, java_vm, vm.get_or_initialize_class("java/io/IOException")?);

        if let Some(file_input_stream) = obj{
            let path = VM::extract_string_from_object(&file_input_stream.get_field(2))?;

            let existing_file = vm.currently_open_files.borrow_mut().remove(&path);
            if let Some((content, index)) = existing_file {
                //file: len 20, i 5
                //buffer: blen 30, o 10, length 20
                //start = 10, end = 25 = 10 + min(30 - 10, 20 - 5)

                let start = offset as usize;
                let end = start + std::cmp::min(length as usize, content.len() - index);
                //println!("start={}, end={}, length={}, readable_bytes={}", start, end, length, content.len() - index);
                (start..end).for_each(|i| data.set_element(i, Value::Integer(content[i - start + index] as i32)));

                let new_index = index + end - start;
                if new_index > index{
                    if new_index == content.len(){
                        //read >0 bytes to end
                        vm.currently_open_files.borrow_mut().insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes to end");
                        non_failing_some(Value::Integer((new_index - index) as i32))
                    } else {
                        //read >0 bytes
                        vm.currently_open_files.borrow_mut().insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes");
                        non_failing_some(Value::Integer((end - start) as i32))
                    }
                } else {
                    if new_index == content.len(){
                        //read 0 bytes from end to end
                        vm.currently_open_files.borrow_mut().insert(path.clone(), (content, new_index));
                        //println!("read 0 bytes from end to end");
                        non_failing_some(Value::Integer(-1))
                    } else {
                        //read 0 bytes
                        vm.currently_open_files.borrow_mut().insert(path.clone(), (content, new_index));
                        //println!("read 0 bytes");
                        non_failing_some(Value::Integer(0))
                    }
                }

                //println!("{:?}", &content[start..end]);
                /*if *index == content.len()-1{
                    vm.currently_open_files.remove(&path);
                    Ok(Some(Value::Integer(-1)))
                } else {
                    *index += end - start;
                    Ok(Some(Value::Integer((end - start) as i32)))
                }*/
            } else {
                let exception_class_name = String::from("java/io/IOException");
                let exception_message = format!("File {} was not found", path);

                let exception_object = vm.try_new_object("java/io/IOException")?;
                let details = wrap_init!(vm, java_vm, vm.new_string_object(exception_message.as_str())?);
                //detailsMessage
                exception_object.set_field(2, Value::Reference(details));
                
                let prev = vm.caught_exception.replace(Some((
                    exception_message,
                    String::from("java/io/FileInputStream.readBytes([BII)I"),
                    Value::Reference(exception_object)
                )));
                assert!(prev.is_none());
                Ok(VMResultType::ExceptionThrown)
            }
        } else {
            Err(VmError::ValidationError("Expected an object reference".to_string()))
        }
    } else {
        Err(VmError::ValidationError("Expected a byte array, integer and integer as args".to_string()))
    }
}

fn delegate_get_file_system<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let class_name = match env::consts::OS {
        "linux" => "rjvm/io/UnixFileSystem",
        "windows" => "rjvm/io/WinFileSystem",
        _ => unimplemented!(),
    };
    let file_system = wrap_init!(vm, java_vm, vm.new_object(class_name)?);

    non_failing_some(Value::Reference(file_system))
}

fn delegate_last_modified_time<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(Value::Reference(path_val)) = args.get(0){
        let string_val = path_val.get_field(1);
        let path = VM::extract_string_from_object(&string_val)?;
        let path = Path::new(&path);
        let last_modified = path.metadata().map(|m| m.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64).unwrap_or(0);
        non_failing_some(Value::Long(last_modified))
    } else {
        Err(VmError::ValidationError("Expected file as parameter".to_string()))
    }
}

const BA_EXISTS: i32 = 1;
const BA_REGULAR: i32 = 2;
const BA_DIRECTORY: i32 = 4;
const BA_HIDDEN: i32 = 8;

fn delegate_get_boolean_attribute<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let path = if let Some(Value::Reference(path_val)) = args.get(0){
        let string_val = path_val.get_field(1);
        VM::extract_string_from_object(&string_val)?
    } else {
        String::new()
    };
    let path = Path::new(&path);
    let mut attributes = 0;
    if path.exists(){
        attributes |= BA_EXISTS;
        if path.is_dir(){
            attributes |= BA_DIRECTORY;
        }
    }
    println!("HILFE {:?} ({}), {}", path, attributes, attributes & BA_EXISTS);
    non_failing_some(Value::Integer(attributes))
}

fn delegate_canonicalize0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    debug!("canonicalize0");
    if let Some(string) = args.get(0){
        let path = VM::extract_string_from_object(string)?;
        let path = Path::new(&path);
        let path = path.canonicalize().unwrap().into_os_string().into_string().unwrap();
        let new_path = wrap_init!(vm, java_vm, vm.new_string_object(path.as_str())?);
        non_failing_some(Value::Reference(new_path))
    } else {
        Err(VmError::ValidationError("Can't canonicalize 0 arguments".to_string()))
    }
}

fn delegate_get_final_path0<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(string) = args.get(0){
        //TODO only valid for windows
        let path = VM::extract_string_from_object(string)?;
        if path.starts_with("\\\\?\\"){
            let is_unc = path.starts_with("\\\\?\\UNC");
            let path = if is_unc {
                path.strip_prefix("\\\\?\\UNC")
            } else {
                path.strip_prefix("\\\\?\\")
            }.unwrap().to_string();
            let new_path = wrap_init!(vm, java_vm, vm.new_string_object(path.as_str())?);
            non_failing_some(Value::Reference(new_path))
        } else {
            Err(VmError::ValidationError(format!("Path not starting with right prefix: {:?}", path)))
        }
    } else {
        Err(VmError::ValidationError("Can't getFinalPath0 with 0 arguments".to_string()))
    }
}

fn delegate_init_unix_fs_dispatcher<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Integer(0))
}

fn delegate_getcwd<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let current_working_dir = env::current_dir().unwrap();
    debug!("getcwd -> '{}'", current_working_dir.display());
    let bytes = current_working_dir.into_os_string().as_encoded_bytes().iter().map(|b| Value::Integer(*b as i32)).collect::<Vec<_>>();
    let path_ref = wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(bytes.clone()))?);
    non_failing_some(Value::Reference(path_ref))
}

fn delegate_init_vm<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let vm_class_id = vm.find_class_by_name("sun/misc/VM").unwrap().id;
    /*let arg1 = wrap_init!(vm, java_vm, vm.new_string_object("java.lang.Integer.IntegerCache.high".to_string())?);
    let arg2 = wrap_init!(vm, java_vm, vm.new_string_object("127".to_string())?);
    let static_vm_object = vm.get_static_class_object(vm_class_id).unwrap();
    let properties_object = static_vm_object.get_field(11).expect_reference()?;

    let save_properties_method = vm.try_resolve_class_method("sun/misc/VM", "saveAndRemoveProperties", "(Ljava/util/Properties;)V")?;
    let frame2 = vm.call_stack.create_and_push_call_frame(save_properties_method, None, vec![Value::Reference(properties_object)], false);
    let properties_set_method = vm.try_resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let frame1 = vm.call_stack.create_and_push_call_frame(properties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)], false);*/
    //Ok(VMResultType::NeedsClassInit(vec![(), ()], false))
    non_failing_none()
}

fn delegate_vm_supports_cs8<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Integer(0))
}

fn delegate_find_signal<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    if let Some(string) = args.get(0){
        let name = VM::extract_string_from_object(string)?;
        let result = match name.as_str() {
            "HUP"  =>  1,
            "INT"  =>  2,
            "TERM" => 15,
            _      => -1
        };
        debug!("Signal name: {} {}", name, result);
        if result > 0{
            return non_failing_some(Value::Integer(result))
        }
    }
    unimplemented!();
    non_failing_some(Value::Integer(0))
}

fn delegate_handle0<'a>(_: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    non_failing_some(Value::Long(0))
}

fn delegate_lookup_cache_urls<'a>(vm: &VM<'a>, _: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    //FIXME add cache, idk how to get this
    non_failing_some(vm.null())
}

fn delegate_perf_create_long<'a>(vm: &VM<'a>, java_vm: &JavaVM, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<Option<Value<'a>>>{
    let class_name = "java/nio/DirectByteBuffer";
    let byte_buffer = wrap_init!(vm, java_vm, vm.new_object(class_name)?);
    let constructor = vm.resolve_class_method(class_name, "<init>", "(JI)V")?;
    let frame_index = vm.call_stack.frames.borrow().len() - 1;
    let addr = vm.unsafe_allocator.allocate_memory(8);
    vm.call_stack.create_and_push_call_frame(constructor, Some(byte_buffer), vec![Value::Long(addr), Value::Dummy, Value::Integer(8)], false);
    let res = vm.invoke_frames_until(java_vm, frame_index as isize)?;
    if let VMResultType::Successful(None) = res{
        non_failing_some(Value::Reference(byte_buffer))
    } else {
        Err(VmError::ValidationError("Error when calling constructor".to_string()))
    }
}

/*
#[cfg(test)]
mod tests{
    use std::cell::RefCell;
    use log::{error, info, Level, LevelFilter};
    use crate::field_info::{FieldType, PrimitiveType};
    use crate::get_or_init;
    use crate::vm::class::ClassAndMethod;
    use crate::vm::class_path::ClassPath;
    use crate::vm::value::{Reference, Value};
    use crate::vm::VM;

    fn setup<'a>() -> VM<'a>{
        simple_logger::SimpleLogger::new().with_level(LevelFilter::Trace).without_timestamps().init().unwrap();
        //simple_logger::init().unwrap();
        let mut class_path = ClassPath::default();
        class_path.push("resources;resources/rt.jar;resources/LogicSim.jar;resources/lib/unix;resources/lib").expect("TODO: panic message");

        let mut vm = VM::new(class_path);
        vm
    }

    fn array_copy_setup(src_index: i32, dst_index: i32, length: i32){
        let mut vm = setup();
        let test_method = vm.resolve_class_method("java/lang/System", "arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V");
        assert!(test_method.is_ok());
        let test_method = test_method.unwrap();

        let src_array = vm.try_new_array(10, FieldType::Primitive(PrimitiveType::Integer), RefCell::new(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
            Value::Null, Value::Null, Value::Null, Value::Null,Value::Null,Value::Null,
        ]));
        assert!(src_array.is_ok());
        let src_array = src_array.unwrap();

        let dst_array = vm.try_new_array(10, FieldType::Primitive(PrimitiveType::Integer), RefCell::new(vec![Value::Null; 10]));
        assert!(dst_array.is_ok());
        let dst_array = dst_array.unwrap();

        println!("src: {:?}", src_array);
        println!("dst: {:?}", dst_array);

        let res = vm.invoke_new_frame(test_method, None, vec![
            Value::Reference(src_array), Value::Integer(src_index), Value::Reference(dst_array), Value::Integer(dst_index), Value::Integer(length)
        ]);
        assert!(res.is_ok());
        let res = res.unwrap().to_option();
        assert!(res.is_none());

        for i in 0..length{
            assert_eq!(src_array.get_element((src_index + i)  as usize), dst_array.get_element((dst_index + i)  as usize));
        }
        println!();
        println!("src: {:?}", src_array);
        println!("dst: {:?}", dst_array);
    }

    #[test]
    fn test_array_copy_1(){
        array_copy_setup(0, 0, 10);
    }

    #[test]
    fn test_array_copy_2() {
        array_copy_setup(1, 2, 2);
    }

    #[test]
    fn test_read_bytes() {
        let mut vm = setup();
        let test_method = vm.resolve_class_method("java/io/FileInputStream", "readBytes", "([BII)I");
        println!("Test: {:?}", test_method);
        assert!(test_method.is_ok());
        let test_method = test_method.unwrap();

        let dst_array = vm.try_new_array(32, FieldType::Primitive(PrimitiveType::Integer), RefCell::new(vec![Value::Null; 32]));
        assert!(dst_array.is_ok());
        let dst_array = dst_array.unwrap();

        let file_input_stream_obj = vm.try_new_object("java/io/FileInputStream");
        assert!(file_input_stream_obj.is_ok());
        let file_input_stream_obj = file_input_stream_obj.unwrap();

        let path = vm.new_string_object("read_test.txt".to_string());
        assert!(path.is_ok());
        let path = path.unwrap();
        file_input_stream_obj.set_field(2, Value::Reference(path));

        let res = vm.invoke_new_frame(test_method.clone(), Some(file_input_stream_obj), vec![
            Value::Reference(dst_array), Value::Integer(0), Value::Integer(21)
        ]);
        //println!("array: {:?}", dst_array);

        assert!(res.is_ok());
        let res = res.unwrap().to_option();
        assert!(res.is_some());
        if let Some(Value::Integer(read)) = res{
            assert_eq!(read, 21);
        } else {
            assert!(false);
        }

        let res = vm.invoke_new_frame(test_method.clone(), Some(file_input_stream_obj), vec![
            Value::Reference(dst_array), Value::Integer(21), Value::Integer(0)
        ]);
        //println!("array: {:?}", dst_array);

        assert!(res.is_ok());
        let res = res.unwrap().to_option();
        assert!(res.is_some());
        if let Some(Value::Integer(read)) = res{
            assert_eq!(read, 0);
        } else {
            assert!(false);
        }

        let res = vm.invoke_new_frame(test_method.clone(), Some(file_input_stream_obj), vec![
            Value::Reference(dst_array), Value::Integer(21), Value::Integer(1)
        ]);
        //println!("array: {:?}", dst_array);

        assert!(res.is_ok());
        let res = res.unwrap().to_option();
        assert!(res.is_some());
        if let Some(Value::Integer(read)) = res{
            assert_eq!(read, 1);
        } else {
            assert!(false);
        }

        let res = vm.invoke_new_frame(test_method.clone(), Some(file_input_stream_obj), vec![
            Value::Reference(dst_array), Value::Integer(22), Value::Integer(30)
        ]);
        //println!("array: {:?}", dst_array);

        assert!(res.is_ok());
        let res = res.unwrap().to_option();
        assert!(res.is_some());
        if let Some(Value::Integer(read)) = res{
            assert_eq!(read, -1);
        } else {
            assert!(false);
        }
    }
}*/