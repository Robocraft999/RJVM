use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use libloading::{library_filename, Library, Symbol};
use log::{debug, trace, warn};
use crate::error::ClassParseError;
use crate::field_info::{get_class_descriptor, FieldType, PrimitiveType};
use crate::get_or_init;
use crate::method_info::MethodDescriptor;
use crate::vm::class::{ClassAndMethod, ClassId, ClassRef};
use crate::vm::java_error::JavaError;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VM, VmError};
use crate::vm::call_frame::CallFrame;
use crate::vm::callstack::CallStack;
use crate::vm::java_error::JavaError::JavaExceptionThrown;
use crate::vm::result::{VMPartialResult, VMResultType};

pub struct NativeMethodRegistry<'a>{
    methods: Vec<NativeMethod<'a>>
}

impl <'a>NativeMethodRegistry<'a>{
    pub fn new() -> Self{
        Self{
            methods: Vec::new()
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

    pub fn invoke(vm: &VM<'a>, class_and_method: &ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Option<VMPartialResult<'a, Option<Value<'a>>>>{
        for method in &vm.native_method_registry.methods{
            if method.method_name == class_and_method.method.name && method.method_descriptor == class_and_method.method.descriptor && class_and_method.class.name == method.class_name{
                let needed_arg_count = class_and_method.method.descriptor.args.len();
                let provided_arg_count = args.iter().filter(|v| v != &&Value::Dummy).count();
                if needed_arg_count == provided_arg_count{
                    return Some((method.delegate)(vm, class_and_method.class, object, args))
                }
                return Some(Err(VmError::ValidationError(format!("expected {} args but got: {}:{:?}", needed_arg_count, provided_arg_count, args))))
            }
        }
        //Some(Err(VmError::JavaException(JavaError::MethodNotFoundException(class_and_method.method.name.clone()))))
        None
    }
}

pub struct NativeMethod<'a>{
    class_name: String,
    method_name: String,
    method_descriptor: MethodDescriptor,
    delegate: NativeMethodDelegate<'a>
}

type NativeMethodDelegate<'a> = fn(&VM<'a>, ClassRef<'a>, Option<Reference<'a>>, Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>;

pub fn register_all_natives(registry: &mut NativeMethodRegistry){
    registry.register("Test", "nop3", "()I", |_, _, _, _| non_failing_some(Value::Integer(-1)));
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
    registry.register("java/lang/Class", "getModifiers", "()I", delegate_get_class_modifiers);
    registry.register("java/lang/Class", "getSuperclass", "()Ljava/lang/Class;", delegate_get_super_class);
    registry.register("java/lang/Class", "forName0", "(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;", delegate_for_name0);
    registry.register("java/lang/Class", "isInterface", "()Z", delegate_is_interface);
    registry.register("java/lang/Class", "isArray", "()Z", delegate_is_array);
    registry.register("java/lang/Class", "isPrimitive", "()Z", delegate_is_primitive);
    registry.register("java/lang/Class", "isAssignableFrom", "(Ljava/lang/Class;)Z", delegate_is_assignable_from);
    registry.register("java/lang/ClassLoader", "findLoadedClass0", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_loaded_class0);
    registry.register("java/lang/ClassLoader", "findBootstrapClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_bootstrap_class);
    registry.register("java/lang/ClassLoader$NativeLibrary", "load", "(Ljava/lang/String;)V", delegate_native_lib_load);
    registry.register("java/lang/Float", "floatToRawIntBits", "(F)I", delegate_float_to_raw_bits);
    registry.register("java/lang/Double", "doubleToRawLongBits", "(D)J", delegate_double_to_raw_bits);
    registry.register("java/lang/Object", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Object", "hashCode", "()I", delegate_hashcode);
    registry.register("java/lang/Object", "clone", "()Ljava/lang/Object;", delegate_clone);
    registry.register("[Ljava/lang/Object;", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Throwable", "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    //registry.register("sun/misc/Unsafe", "registerNatives", "()V", delegate_nop);
    registry.register("sun/misc/Unsafe", "arrayBaseOffset", "(Ljava/lang/Class;)I", delegate_array_base_offset);
    registry.register("sun/misc/Unsafe", "arrayIndexScale", "(Ljava/lang/Class;)I", delegate_array_index_scale);
    registry.register("sun/misc/Unsafe", "addressSize", "()I", delegate_address_size);
    registry.register("sun/misc/Unsafe", "objectFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_object_field_offset);
    registry.register("sun/misc/Unsafe", "staticFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_static_field_offset);
    registry.register("sun/misc/Unsafe", "getObjectVolatile", "(Ljava/lang/Object;J)Ljava/lang/Object;", delegate_get_object_volatile);
    registry.register("sun/misc/Unsafe", "staticFieldBase", "(Ljava/lang/reflect/Field;)Ljava/lang/Object;", delegate_static_field_base);
    registry.register("sun/misc/Unsafe", "compareAndSwapObject", "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z", delegate_compare_and_swap_object);
    registry.register("sun/misc/Unsafe", "compareAndSwapInt", "(Ljava/lang/Object;JII)Z", delegate_compare_and_swap_int);
    registry.register("sun/misc/Unsafe", "compareAndSwapLong", "(Ljava/lang/Object;JJJ)Z", delegate_compare_and_swap_long);
    registry.register("sun/misc/Unsafe", "allocateMemory", "(J)J", delegate_allocate_memory);
    registry.register("sun/misc/Unsafe", "putLong", "(JJ)V", delegate_put_long);
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
    registry.register("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;", delegate_get_stack_access_control_context);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/lang/String", "intern", "()Ljava/lang/String;", delegate_string_intern);
    registry.register("sun/reflect/NativeConstructorAccessorImpl", "newInstance0", "(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_new_instance0);
    registry.register("java/io/FileOutputStream", "writeBytes", "([BIIZ)V", delegate_write_bytes);
    //registry.register("java/io/FileInputStream", "initIDs", "()V", delegate_nop);
    registry.register("java/io/FileInputStream", "readBytes", "([BII)I", delegate_read_bytes);
    registry.register("java/io/FileSystem", "getFileSystem", "()Ljava/io/FileSystem;", delegate_get_file_system);
    registry.register("rjvm/io/UnixFileSystem", "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register("rjvm/io/UnixFileSystem", "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register("rjvm/io/WinFileSystem",  "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register("rjvm/io/WinFileSystem", "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register("rjvm/io/WinFileSystem", "getFinalPath0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_get_final_path0);
    registry.register("sun/nio/fs/UnixNativeDispatcher", "init", "()I", delegate_init_unix_fs_dispatcher);
    registry.register("sun/nio/fs/UnixNativeDispatcher", "getcwd", "()[B", delegate_getcwd);
    registry.register("sun/misc/VM", "initialize", "()V", delegate_init_vm);
    registry.register("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z", delegate_vm_supports_cs8);
    registry.register("sun/misc/Signal", "findSignal", "(Ljava/lang/String;)I", delegate_find_signal);
    registry.register("sun/misc/Signal", "handle0", "(IJ)J", delegate_handle0);
}

fn non_failing_some<'a>(value: Value<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
    Ok(VMResultType::NativeOk(Some(value)))
}

fn non_failing_none<'a>() -> VMPartialResult<'a, Option<Value<'a>>> {
    Ok(VMResultType::NativeOk(None))
}

fn delegate_nop<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_none()
}

fn delegate_nano_time<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    non_failing_some(Value::Long(nanos))
}
fn delegate_millis_time<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    non_failing_some(Value::Long(millis))
}

fn delegate_identity_hash_code<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(object)) = args.get(0){
        let addr = &object as *const _;
        let addr = addr as i32;
        trace!("HASH: {addr}");
        non_failing_some(Value::Integer(addr))
    } else {
        Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
    }
}

fn delegate_set_out<'a>(vm: &VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_set_err<'a>(vm: &VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_arraycopy<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_init_system_props<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let properties_object = args.get(0).unwrap().expect_reference()?;
    let mut props = vec![
        ("file.encoding", "UTF-8".to_string()),
        ("line.separator", "\n".to_string()),
        ("java.lang.Integer.IntegerCache.high", "127".to_string()),
        //("sun.boot.library.path", "/home/admin/.jdks/temurin-22.0.1/lib".to_string()),
        ("sun.boot.library.path", "/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/".to_string()),
        ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
        ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
        ("os.name", "Linux".to_string()),
    ];
    if env::consts::OS == "windows"{
        props = vec![
            ("file.encoding", "UTF-8".to_string()),
            ("line.separator", "\r\n".to_string()),
            ("java.lang.Integer.IntegerCache.high", "127".to_string()),
            ("sun.boot.library.path", "C:\\Users\\Admin\\.jdks\\azul-22.0.1\\bin".to_string()),
            ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
            ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
            ("os.name", "Windows".to_string()),
        ];
    }
    let properties_set_method = vm.try_resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let frames: Vec<()> = props.into_iter().map(|(key, value)| {
        //FIXME could be bad to unwrap
        let arg1 = vm.try_new_string_object(key.to_string()).unwrap();
        let arg2 = vm.try_new_string_object(value).unwrap();
        vm.call_stack.create_and_push_call_frame(properties_set_method.clone(), Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)], false)
    }).collect();
    //Ok(VMResultType::NeedsClassInit(frames, false))
    non_failing_some(Value::Null)
}

fn delegate_system_map_library_name<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(string) = args.get(0) {
        let name = VM::extract_string_from_object(string)?;
        let new_name = match env::consts::OS{
            "windows" => name + ".dll",
            "linux" => format!("lib{name}.so"),
            _ => name
        };
        non_failing_some(Value::Reference(get_or_init!(vm.new_string_object(new_name)?)))
    } else {
        Err(VmError::ValidationError(format!("Expected Reference but found '{:?}'", args.get(0))))
    }
}

fn delegate_get_primitive_class<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let string = VM::extract_string_from_object(args.get(0).unwrap())?;
    match string.as_str() {
        "int"     => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(  "java/lang/Integer".to_string())?))),
        "long"    => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(     "java/lang/Long".to_string())?))),
        "short"   => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(    "java/lang/Short".to_string())?))),
        "char"    => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name("java/lang/Character".to_string())?))),
        "byte"    => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(     "java/lang/Byte".to_string())?))),
        "float"   => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(    "java/lang/Float".to_string())?))),
        "double"  => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(   "java/lang/Double".to_string())?))),
        "boolean" => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(  "java/lang/Boolean".to_string())?))),
        "void"    => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(     "java/lang/Void".to_string())?))),
        _ => Err(VmError::ValidationError(format!("Expected extractable string")))
    }
}

fn delegate_get_component_type<'a>(vm: &VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("getComponentType \n'{:?}'\n'{:?}'", class_object, args);
    let class_name = VM::extract_class_name_from_class_object(class_object.unwrap())?;
    //let field_type = field_type_from_str(class_name.as_str());
    debug!("getComponentType '{:?}'", class_name);

    let array_class = get_or_init!(vm.get_or_resolve_class(class_name.as_str())?);
    if let Some(array_info) = &array_class.array_info{
        let component_class_object = get_or_init!(vm.new_class_object_by_name(array_info.component_type.to_class_name())?);
        non_failing_some(Value::Reference(component_class_object))
    } else {
        Err(VmError::ValidationError(format!("Expected Array object but found '{:?}'", class_object)))
    }
}

fn delegate_get_classloader<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //TODO check
    debug!("getClassLoader0");
    non_failing_some(Value::Null)
}

fn delegate_desired_assertion_status<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //TODO check
    debug!("desiredAssertionStatus0");
    non_failing_some(Value::Integer(1))
}

fn delegate_get_declared_fields0<'a>(vm: &VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("getDeclaredFields");
    if let Some(clazz) = class_object {
        let class_name = VM::extract_class_name_from_class_object(clazz)?;
        debug!("class name: {}", class_name);
        let mut content = Vec::new();
        for field in get_or_init!(vm.get_or_resolve_class(class_name.as_str())?).fields.iter(){
            let java_field = get_or_init!(vm.new_object("java/lang/reflect/Field")?);
            //name
            java_field.set_field(6, Value::Reference(get_or_init!(vm.new_string_object(field.name.clone())?)));
            //clazz
            java_field.set_field(4, Value::Reference(clazz));
            //modifiers
            java_field.set_field(8, Value::Integer(field.flags.iter().cloned().map(|flag| flag as u16 as i32).reduce(|flag1, flag2| flag1 | flag2).unwrap_or(0)));
            //type
            let type_class_object = get_or_init!(vm.new_class_object_by_name(field.field_type.to_class_name())?);
            java_field.set_field(7, Value::Reference(type_class_object));
            debug!("field name: {}", field.name);
            content.push(Value::Reference(java_field));
        }
        for field in content.iter(){
            if let Value::Reference(java_field) = field {
                warn!("field : {:?}", java_field);
                if let ReferenceType::Object(fields) = &java_field.reference_type{
                    for field_field in fields.borrow().iter(){
                        warn!("field_: {:?}", field_field);
                    }
                }
            }
        }
        non_failing_some(Value::Reference(get_or_init!(vm.new_array(1, FieldType::Object("java/lang/reflect/Field".to_string()).to_array_field_type(1), RefCell::new(content))?)))
    } else {
        //FIXME i dont know if this should be none
        non_failing_none()
    }
}

fn delegate_get_declared_constructors0<'a>(vm: &VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("getDeclaredConstructors");
    if let Some(class_ref) = class_object{
        let class = get_or_init!(vm.extract_class_from_class_object(class_ref)?);
        let mut content = Vec::new();
        for constructor in class.get_constructors().iter(){
            let java_constructor = get_or_init!(vm.new_object("java/lang/reflect/Constructor")?);

            //clazz
            java_constructor.set_field(4, Value::Reference(class_ref));

            let mut parameters = Vec::new();
            for field_type in constructor.descriptor.args.iter(){
                let parameter_class = get_or_init!(vm.new_class_object_by_name(field_type.to_class_name())?);
                parameters.push(Value::Reference(parameter_class));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = constructor.exceptions.clone(){
                for exception in exception_vec.0{
                    let parameter_class = get_or_init!(vm.new_class_object_by_name(exception)?);
                    exceptions.push(Value::Reference(parameter_class));
                }
            }
            //parameterTypes
            java_constructor.set_field(6, Value::Reference(get_or_init!(vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(parameters))?)));
            
            //exceptionTypes
            java_constructor.set_field(7, Value::Reference(get_or_init!(vm.new_array(1, FieldType::Object("java/lang/Class".to_string()).to_array_field_type(1), RefCell::new(exceptions))?)));

            let flags = constructor.flags.iter().map(|flag| flag.clone() as u16).reduce(|flag1, flag2| flag1 | flag2).unwrap_or(0);
            //modifiers
            java_constructor.set_field(8, Value::Integer(flags as i32));

            content.push(Value::Reference(java_constructor));
        }
        non_failing_some(Value::Reference(get_or_init!(vm.new_array(1, FieldType::Object("java/lang/reflect/Constructor".to_string()).to_array_field_type(1), RefCell::new(content))?)))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_get_class_modifiers<'a>(vm: &VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(obj) = class_object{
        let class = get_or_init!(vm.extract_class_from_class_object(obj)?);
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_get_super_class<'a>(vm: &VM<'a>, _: ClassRef<'a>, this: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(obj) = this {
        let class = get_or_init!(vm.extract_class_from_class_object(obj)?);
        match class.superclass {
            Some(super_class) => {
                let super_class_object = get_or_init!(vm.new_class_object_by_name(super_class.name.clone())?);
                non_failing_some(Value::Reference(super_class_object))
            }
            None => non_failing_some(Value::Null)
        }

    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_for_name0<'a>(vm: &VM<'a>,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("forName0");
    if let Some(name) = args.get(0) {
        let name = VM::extract_string_from_object(&name)?;
        let name = name.replace(".", "/");
        match vm.get_or_resolve_class(&name){
            Ok(_) => non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(name)?))),
            Err(VmError::ParseError(ClassParseError::ResolveError(_))) => {
                let exception_class_name = String::from("java/lang/ClassNotFoundException");
                let exception_message = format!("Class {} was not found", name);

                let exception_class = get_or_init!(vm.get_or_resolve_class(&exception_class_name)?);
                let exception_object = vm.try_new_object(&exception_class_name)?;
                //let init = vm.get_class_method(exception_class, "<init>", "(Ljava/lang/String;)V")?;
                let details = get_or_init!(vm.new_string_object(exception_message.clone())?);
                //detailsMessage
                exception_object.set_field(2, Value::Reference(details));
                //vm.call_stack.create_and_push_call_frame(init, Some(exception_object), vec![Value::Reference(details)], false);
                Ok(VMResultType::NativeException(
                    VmError::JavaException(
                        JavaExceptionThrown(
                            exception_class_name,
                            exception_message,
                            String::from("java/lang/Class.forName0(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;")
                        )
                    ),
                    Value::Reference(exception_object)
                ))
            }
            Err(err) => Err(err)
        }
        //let class = vm.find_class_by_name(name)?;

    } else {
        Err(VmError::ValidationError("no name".to_string()))
    }
}

fn delegate_is_interface<'a>(vm: &VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("isInterface {:?}", obj);
    if let Some(obj) = obj {
        let class = get_or_init!(vm.extract_class_from_class_object(obj)?);
        non_failing_some(Value::from(class.is_interface()))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_array<'a>(vm: &VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("isArray {:?}", obj);
    if let Some(obj) = obj {
        non_failing_some(Value::from(obj.is_array()))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_primitive<'a>(vm: &VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_is_assignable_from<'a>(vm: &VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("isAssignableFrom\nthis: {:?}\nfrom: {:?}", obj, args);
    if let (Some(object), Some(Value::Reference(other))) = (obj, args.get(0)) {
        let this_class = get_or_init!(vm.extract_class_from_class_object(object)?);
        let from_class = get_or_init!(vm.extract_class_from_class_object(other)?);
        non_failing_some(Value::from(vm.unchecked_check_if_subclass_of(this_class.name.as_str(), from_class.name.as_str())?))
    } else {
        Err(VmError::ValidationError("expected a class reference".to_string()))
    }
}

fn delegate_find_loaded_class0<'a>(vm: &VM<'a>,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("findLoadedClass0 {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = VM::extract_string_from_object(&str_object)?;
        if vm.class_manager.find_class_by_name(class_name.as_str()).is_some() {
            non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(class_name)?)))
        } else {
            non_failing_some(Value::Null)
        }
    } else {
        Err(VmError::ValidationError("expected a string reference".to_string()))
    }
}

fn delegate_find_bootstrap_class<'a>(vm: &VM<'a>,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("findBootstrapClass {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = VM::extract_string_from_object(&str_object)?;
        if vm.class_manager.find_class_by_name(class_name.as_str()).is_some() {
            non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(class_name)?)))
        } else {
            non_failing_some(Value::Null)
        }
    } else {
        Err(VmError::ValidationError("expected a string reference".to_string()))
    }
}

fn delegate_native_lib_load<'a>(vm: &VM<'a>,  _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("nativeLib::load {:?}", object);
    if let Some(obj) = object {
        //handle
        obj.set_field(0, Value::Long(1));
        let name_field = obj.get_field(3);//args.get(0).unwrap();
        let name = VM::extract_string_from_object(&name_field)?;
        println!("name: {name}");

        /*unsafe {
            let lib_name = name;
            //let lib_name = library_filename(name);
            println!("name: {lib_name:?}");
            let lib = Library::new(lib_name).unwrap(); // Load the "hello_world" library
            let func: Symbol<fn()> = lib.get(b"JNI_OnLoad").unwrap(); // Get the function pointer

            func() // Call the function
        }*/

        non_failing_none()
    } else {
        Err(VmError::ValidationError("this is null".to_string()))
    }
}

fn delegate_float_to_raw_bits<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Float(value)) = args.get(0){
        return non_failing_some(Value::Integer(value.to_bits() as i32))
    }
    Err(VmError::ValidationError(format!("Expected float")))
}

fn delegate_double_to_raw_bits<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Double(value)) = args.get(0){
        return non_failing_some(Value::Long(value.to_bits() as i64))
    }
    Err(VmError::ValidationError(format!("Expected double")))
}

fn delegate_get_class<'a>(vm: &VM<'a>, class: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //TODO check
    debug!("getClass");
    if let Some(obj) = object {
        debug!("{} obj: {:?}", class.name, obj.class_name);
        let class_object = get_or_init!(vm.new_class_object_by_name(obj.class_name.clone())?);
        non_failing_some(Value::Reference(class_object))
    } else {
        Err(VmError::ValidationError("Object is Null".to_string()))
    }
}

fn delegate_hashcode<'a>(_: &VM<'a>, _: ClassRef<'a>, reference: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //FIXME hash string not address
    if let Some(obj) = reference{
        let addr = &obj as *const _;
        let addr = addr as i32;
        trace!("HASHCODE: {addr}");
        non_failing_some(Value::Integer(addr))
    } else {
        Err(VmError::ValidationError("Expected object".to_string()))
    }
}

fn delegate_clone<'a>(vm: &VM<'a>, _: ClassRef<'a>, reference: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("clone");
    if let Some(obj) = reference{
        if obj.is_array(){
            if let ReferenceType::Array(dims, component_type, content) = &obj.reference_type{
                debug!("Cloning array: {:?}", reference);
                let new_array = Value::Reference(get_or_init!(vm.new_array(*dims, component_type.clone().to_array_field_type(*dims), content.clone())?));
                non_failing_some(new_array)
            } else {
                Err(VmError::ValidationError("Expected array to be cloned".to_string()))
            }
        } else {
            if let ReferenceType::Object(content) = &obj.reference_type{
                debug!("Cloning object: {:?}", reference);
                let mut new_object = get_or_init!(vm.new_object(obj.class_name.as_str())?);
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

fn delegate_fill_in_stacktrace<'a>(_: &VM<'a>, _ : ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(receiver) = object{
        return non_failing_some(Value::Reference(receiver));
    }
    Err(VmError::ValidationError("Expected a Throwable".to_string()))
}

fn delegate_array_base_offset<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(class)) = args.get(0){
        non_failing_some(Value::Integer(16))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_array_index_scale<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(class)) = args.get(0){
        non_failing_some(Value::Integer(1))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_address_size<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(8))
}

fn delegate_object_field_offset<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //FIXME calc real offset
    debug!("delegate_object_field_offset: '{:?}'", args);
    if let Some(field) = args.get(0){
        let field_ref = field.expect_reference()?;
        let clazz = field_ref.get_field(4).expect_reference()?;
        let class_ref = get_or_init!(vm.extract_class_from_class_object(clazz)?);
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

fn delegate_static_field_offset<'a>(vm: &VM<'a>, class : ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //non_failing_some(Value::Long(0))
    //TODO check if needed
    delegate_object_field_offset(vm, class, object, args)
}

fn delegate_get_object_volatile<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("get_object_volatile args: {:?}", args);
    if let (Some(Value::Reference(o)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize  - 16));
        }
        let field_value = if o.class_name == "java/lang/Class"{
            let class_ref = get_or_init!(vm.extract_class_from_class_object(o)?);
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

fn delegate_static_field_base<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(field_object_value) = args.get(0){
        let field_object = field_object_value.expect_reference()?;
        println!("'{:?}'", field_object);
        let class_object = field_object.get_field(4);
        non_failing_some(class_object)
    } else {
        Err(VmError::ValidationError("Expected a field reference".to_string()))
    }
}

fn delegate_compare_and_swap_object<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(1))
}

fn delegate_compare_and_swap_int<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(1))
}

fn delegate_compare_and_swap_long<'a>(_: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(1))
}

fn delegate_allocate_memory<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Long(num)) = args.get(0){
        //return is address in memory
        let ptr = vm.unsafe_allocator.allocate_memory(*num as usize);
        non_failing_some(Value::Long(ptr))
    } else {
        Err(VmError::ValidationError("Expected a long".to_string()))
    }
}

fn delegate_put_long<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    //because args = [Long, Dummy, Long, Dummy]
    if let (Some(Value::Long(ptr)), Some(Value::Long(value))) = (args.get(0), args.get(2)){
        vm.unsafe_allocator.put_long(*ptr, *value);
        non_failing_none()
    } else {
        Err(VmError::ValidationError("Expected a long as address and a long as value".to_string()))
    }
}

fn delegate_get_byte<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Long(ptr)) = args.get(0){
        let byte = vm.unsafe_allocator.get_byte(*ptr);
        Ok(VMResultType::NativeOk(byte.map(|byte| Value::Integer(byte as i32))))
    } else {
        Err(VmError::ValidationError("Expected a long as address".to_string()))
    }
}

fn delegate_put_ordered_object<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("put_ordered_object args: {:?}", args);
    if let (Some(Value::Reference(o)), Some(Value::Long(index)), Some(x)) = (args.get(0), args.get(1), args.get(3)) {
        if o.is_array(){
            o.set_element(*index as usize  - 16, x.clone());
            return non_failing_none();
        }
        if o.class_name == "java/lang/Class"{
            let class_ref = get_or_init!(vm.extract_class_from_class_object(o)?);
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

fn delegate_define_class<'a>(vm: &VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let (Some(class_name_value), Some(Value::Reference(bytes_value)), Some(start), Some(end)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let class_name = VM::extract_string_from_object(class_name_value)?;
        let bytes = if let ReferenceType::Array(_, _, data) = &bytes_value.reference_type{
            data.borrow().iter().map(|val| if let Value::Integer(byte) = val {*byte as u8} else {0}).collect()
        } else {
            Vec::new()
        };
        let (start, end) = (start.expect_int()?, end.expect_int()?);
        let bytes = bytes.into_iter().skip(start as usize).take((end - start) as usize).collect::<Vec<_>>();
        let class_object = get_or_init!(vm.define_class(class_name.as_str(), bytes)?);
        non_failing_some(Value::Reference(class_object))
    } else {
        Err(VmError::ValidationError(format!("define_class: expected string_object, byte array, start and end ints but got: {:?}, {:?}, {:?}, {:?}", args.get(0), args.get(1), args.get(2), args.get(3))))
    }
}

fn delegate_allocate_instance<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(class_object)) = args.get(0){
        let class_name = VM::extract_class_name_from_class_object(class_object)?;
        let object = get_or_init!(vm.new_object(class_name.as_str())?);
        non_failing_some(Value::Reference(object))
    } else {
        Err(VmError::ValidationError(format!("Expected a class reference to allocate but got: {:?}", args)))
    }
}

fn delegate_get_caller_class<'a>(vm: &VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let frame_index = vm.call_stack.frames.borrow().len() - 2 - 1;
    if let Some(frame) = vm.call_stack.frames.borrow().get(frame_index){
        non_failing_some(Value::Reference(get_or_init!(vm.new_class_object_by_name(frame.class_and_method.class.name.clone())?)))
    } else {
        Err(VmError::ValidationError("There is no parent Callframe".to_string()))
    }
}

fn delegate_get_class_access_flags<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(obj)) = args.get(0){
        let class = get_or_init!(vm.extract_class_from_class_object(obj)?);
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_current_thread<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if vm.current_thread.borrow().is_none(){
        let thread = get_or_init!(vm.new_object("java/lang/Thread")?);
        //let thread_init = vm.resolve_class_method("java/lang/Thread", "<init>", "()V")?;
        //vm.invoke(thread_init, Some(thread), vec![])?;
        let name_string = get_or_init!(vm.new_string_object("Main".to_string())?);
        let name_char_array = name_string.get_field(0);

        let group_name = get_or_init!(vm.new_string_object("system".to_string())?);
        let group = get_or_init!(vm.new_object("java/lang/ThreadGroup")?);
        group.set_field(6, Value::Integer(0));
        group.set_field(1, Value::Reference(group_name));
        group.set_field(2, Value::Integer(10));
        group.set_field(0, Value::Null);

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

fn delegate_is_alive<'a>(vm: &VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(object.unwrap().get_field(5))
}

fn delegate_available_processors<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(1))
}

fn delegate_free_memory<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Long(1024 * 1024 * 20))
}

fn delegate_get_stack_access_control_context<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Null)
}

fn delegate_do_privileged<'a>(vm: &VM<'a>, class: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(Value::Reference(action)) = args.get(0){
        let class_name = vm.find_class_by_id(action.class_id).unwrap().name.as_str();
        let run = get_or_init!(vm.resolve_class_method(class_name, "run", "()Ljava/lang/Object;")?);
        vm.call_stack.create_and_push_call_frame(run, Some(action), vec![], true);//TODO check if always no return push
        //Ok(VMResultType::NeedsClassInit(vec![()], false))
        non_failing_none()
        //Ok(vm.invoke_new_frame(run, Some(action), vec![])?)
    } else {
        Err(VmError::ValidationError("Expected a action object reference".to_string()))
    }
}

fn delegate_string_intern<'a>(vm: &VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(obj) = object{
        let content = VM::extract_string_from_object(&Value::Reference(obj))?;
        if vm.string_objects.borrow().contains_key(&content){
            non_failing_some(Value::Reference(vm.string_objects.borrow()[&content]))
        } else {
            non_failing_some(Value::Reference(obj))
        }
    } else {
        Err(VmError::ValidationError("Expected a string object reference".to_string()))
    }
}

fn delegate_new_instance0<'a>(vm: &VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    debug!("newInstance0");
    debug!("{:?}", args);
    if let Some(Value::Reference(constructor)) = args.get(0){
        let clazz = constructor.get_field(4);
        let parameter_types = constructor.get_field(6);
        if let (Value::Reference(class_ref), Value::Reference(parameter_array)) = (clazz, parameter_types){
            if let ReferenceType::Array(_, _, type_content) = &parameter_array.reference_type{
                let class = get_or_init!(vm.extract_class_from_class_object(class_ref)?);
                let mut descriptor = String::from("(");
                for constructor_parameter_type in type_content.borrow().iter(){
                    if let Value::Reference(parameter_type_ref) = constructor_parameter_type {
                        let class = get_or_init!(vm.extract_class_from_class_object(parameter_type_ref)?);
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
                    let object = get_or_init!(vm.new_object(class_and_method.class.name.as_str())?);
                    vm.call_stack.create_and_push_call_frame(class_and_method, Some(object), constructor_args, false);
                    //let return_frame = CallStack::create_returning_frame(vm.find_class_by_id(ClassId(0)).unwrap(), Value::Reference(object));
                    //return Ok(VMResultType::NeedsClassInit(vec![return_frame, frame], false));
                    return non_failing_some(Value::Reference(object))
                }
            }
        }
        non_failing_none()
    } else {
        Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
    }
}

fn delegate_write_bytes<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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


fn delegate_read_bytes<'a>(vm: &VM<'a>, _: ClassRef<'a>, obj: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let (Some(arg0), Some(arg1), Some(arg2)) = (args.get(0), args.get(1), args.get(2)) {
        let data = arg0.expect_reference()?;
        let offset = arg1.expect_int()?;
        let length = arg2.expect_int()?;
        
        let io_exception_class = get_or_init!(vm.get_or_resolve_class("java/io/IOException")?);

        if let Some(file_input_stream) = obj{
            let path = VM::extract_string_from_object(&file_input_stream.get_field(2))?;
            if !vm.currently_open_files.borrow().contains_key(&path){
                //TODO do this on open0()
                let file_content = vm.class_manager.class_path.resolve_file(path.as_str())?;
                if let Some(file_content) = file_content{
                    vm.currently_open_files.borrow_mut().insert(path.clone(), (file_content, 0));
                }
            }

            if let Some((content, index)) = vm.currently_open_files.borrow_mut().remove(&path) {
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
                unimplemented!("see getName0");
                let exception_object = vm.try_new_object("java/io/IOException")?;
                let init = vm.get_class_method(io_exception_class, "<init>", "(Ljava/lang/String;)V")?;
                let details = get_or_init!(vm.new_string_object(format!("File {} was not found", path))?);
                let init_frame = vm.call_stack.create_and_push_call_frame(init, Some(exception_object), vec![Value::Reference(details)], false);
                //let throw_frame = CallStack::create_throwing_frame(vm.find_class_by_id(ClassId(0)).unwrap(), Value::Reference(exception_object));
                //Ok(VMResultType::NeedsClassInit(vec![(), ()], false))
                non_failing_none()
                //Err(VmError::JavaException(JavaError::IOException(format!("File {} was not found", path))))
            }
        } else {
            Err(VmError::ValidationError("Expected an object reference".to_string()))
        }
    } else {
        Err(VmError::ValidationError("Expected a byte array, integer and integer as args".to_string()))
    }
}

fn delegate_get_file_system<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let class_name = match env::consts::OS {
        "linux" => "rjvm/io/UnixFileSystem",
        "windows" => "rjvm/io/WinFileSystem",
        _ => unimplemented!(),
    };
    let file_system = get_or_init!(vm.new_object(class_name)?);
    non_failing_some(Value::Reference(file_system))
}

const BA_EXISTS: i32 = 1;
const BA_REGULAR: i32 = 2;
const BA_DIRECTORY: i32 = 4;
const BA_HIDDEN: i32 = 8;

fn delegate_get_boolean_attribute<'a>(vm: &VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_canonicalize0<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(string) = args.get(0){
        let path = VM::extract_string_from_object(string)?;
        let path = Path::new(&path);
        let path = path.canonicalize().unwrap().into_os_string().into_string().unwrap();
        let new_path = get_or_init!(vm.new_string_object(path)?);
        non_failing_some(Value::Reference(new_path))
    } else {
        Err(VmError::ValidationError("Can't canonicalize 0 arguments".to_string()))
    }
}

fn delegate_get_final_path0<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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
            let new_path = get_or_init!(vm.new_string_object(path)?);
            non_failing_some(Value::Reference(new_path))
        } else {
            Err(VmError::ValidationError(format!("Path not starting with right prefix: {:?}", path)))
        }
    } else {
        Err(VmError::ValidationError("Can't getFinalPath0 with 0 arguments".to_string()))
    }
}

fn delegate_init_unix_fs_dispatcher<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(0))
}

fn delegate_getcwd<'a>(vm: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let current_working_dir = env::current_dir().unwrap();
    debug!("getcwd -> '{}'", current_working_dir.display());
    let bytes = current_working_dir.into_os_string().as_encoded_bytes().iter().map(|b| Value::Integer(*b as i32)).collect::<Vec<_>>();
    let path_ref = get_or_init!(vm.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(bytes))?);
    non_failing_some(Value::Reference(path_ref))
}

fn delegate_init_vm<'a>(vm: &VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let vm_class_id = vm.find_class_by_name("sun/misc/VM".to_owned()).unwrap().id;
    /*let arg1 = get_or_init!(vm.new_string_object("java.lang.Integer.IntegerCache.high".to_string())?);
    let arg2 = get_or_init!(vm.new_string_object("127".to_string())?);
    let static_vm_object = vm.get_static_class_object(vm_class_id).unwrap();
    let properties_object = static_vm_object.get_field(11).expect_reference()?;

    let save_properties_method = vm.try_resolve_class_method("sun/misc/VM", "saveAndRemoveProperties", "(Ljava/util/Properties;)V")?;
    let frame2 = vm.call_stack.create_and_push_call_frame(save_properties_method, None, vec![Value::Reference(properties_object)], false);
    let properties_set_method = vm.try_resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let frame1 = vm.call_stack.create_and_push_call_frame(properties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)], false);*/
    //Ok(VMResultType::NeedsClassInit(vec![(), ()], false))
    non_failing_none()
}

fn delegate_vm_supports_cs8<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Integer(0))
}

fn delegate_find_signal<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
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

fn delegate_handle0<'a>(_: &VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
    non_failing_some(Value::Long(0))
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