use std::cell::RefCell;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{debug, warn};
use crate::field_info::{field_type_from_str, get_class_descriptor, FieldType, PrimitiveType};
use crate::method_info::MethodDescriptor;
use crate::vm::class::{ClassAndMethod, ClassRef};
use crate::vm::java_error::JavaError;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VM, VmError};

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

    pub fn invoke(vm: &mut VM<'a>, class_and_method: &ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Option<Result<Option<Value<'a>>, VmError>>{
        for method in &vm.native_method_registry.methods{
            if method.method_name == class_and_method.method.name && method.method_descriptor == class_and_method.method.descriptor{
                return Some((method.delegate)(vm, class_and_method.class, object, args))
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

type NativeMethodDelegate<'a> = fn(&mut VM<'a>, ClassRef<'a>, Option<Reference<'a>>, Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>;

pub fn register_all_natives(registry: &mut NativeMethodRegistry){
    registry.register("java/lang/System", "nanoTime", "()J", delegate_nano_time);
    registry.register("java/lang/System", "currentTimeMillis", "()J", delegate_millis_time);
    registry.register("java/lang/System", "identityHashCode", "(Ljava/lang/Object;)I", delegate_identity_hash_code);
    registry.register("java/lang/System", "setOut0", "(Ljava/io/PrintStream;)V", delegate_set_out);
    registry.register("java/lang/System", "arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V", delegate_arraycopy);
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
    registry.register("java/lang/Float", "floatToRawIntBits", "(F)I", delegate_float_to_raw_bits);
    registry.register("java/lang/Double", "doubleToRawLongBits", "(D)J", delegate_double_to_raw_bits);
    registry.register("java/lang/Object", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Object", "hashCode", "()I", delegate_hashcode);
    registry.register("java/lang/Throwable", "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    registry.register("sun/misc/Unsafe", "arrayBaseOffset", "(Ljava/lang/Class;)I", delegate_array_base_offset);
    registry.register("sun/misc/Unsafe", "arrayIndexScale", "(Ljava/lang/Class;)I", delegate_array_index_scale);
    registry.register("sun/misc/Unsafe", "addressSize", "()I", delegate_address_size);
    registry.register("sun/misc/Unsafe", "objectFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_object_field_offset);
    registry.register("sun/misc/Unsafe", "compareAndSwapObject", "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z", delegate_compare_and_swap_object);
    registry.register("sun/misc/Unsafe", "compareAndSwapInt", "(Ljava/lang/Object;JII)Z", delegate_compare_and_swap_int);
    registry.register("sun/misc/Unsafe", "allocateMemory", "(J)J", delegate_allocate_memory);
    registry.register("sun/misc/Unsafe", "putLong", "(JJ)V", delegate_put_long);
    registry.register("sun/misc/Unsafe", "getByte", "(J)B", delegate_get_byte);
    registry.register("sun/reflect/Reflection", "getCallerClass", "()Ljava/lang/Class;", delegate_get_caller_class);
    registry.register("sun/reflect/Reflection", "getClassAccessFlags", "(Ljava/lang/Class;)I", delegate_get_class_access_flags);
    registry.register("java/lang/Thread", "currentThread", "()Ljava/lang/Thread;", delegate_current_thread);
    registry.register("java/lang/Thread", "isAlive", "()Z", delegate_is_alive);
    registry.register("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;", delegate_get_stack_access_control_context);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/lang/String", "intern", "()Ljava/lang/String;", delegate_string_intern);
    registry.register("sun/reflect/NativeConstructorAccessorImpl", "newInstance0", "(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_new_instance0);
}

fn delegate_nano_time<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    Ok(Some(Value::Long(nanos)))
}
fn delegate_millis_time<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    Ok(Some(Value::Long(millis)))
}

fn delegate_identity_hash_code<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Reference(object)) = args.get(0){
        let addr = &object as *const _;
        let addr = addr as i32;
        println!("HASH: {addr}");
        Ok(Some(Value::Integer(addr)))
    } else {
        Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
    }
}

fn delegate_set_out<'a>(vm: &mut VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(static_object) = vm.get_static_class_object(class.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_object.set_field(2, Value::Reference(object));
            Ok(None)
        } else {
            Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
        }
    } else {
        Err(VmError::ValidationError(format!("Couldn't find static Object of class {}", class.name)))
    }
}

fn delegate_arraycopy<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let (Some(Value::Reference(ref1)), Some(Value::Integer(src_pos)), Some(Value::Reference(ref2)), Some(Value::Integer(dst_pos))) =
        (args.get(0), args.get(1), args.get(2), args.get(3)){
        if let (Some(Value::Integer(length)), ReferenceType::Array(_, _, src), ReferenceType::Array(_, _, dst)) =
            (args.get(4), &ref1.reference_type, &ref2.reference_type){
            for i in 0..*length {
                let src_index = *src_pos + i;
                let dest_index = *dst_pos + i;
                dst.borrow_mut()[dest_index as usize] = src.borrow()[src_index as usize].clone();
            }
            return Ok(None)
        }
    }
    return Err(VmError::ValidationError("Expected two arrays with indices".to_string()));
}

fn delegate_get_primitive_class<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let string = vm.extract_string_from_object(args.get(0).unwrap())?;
    match string.as_str() {
        "int"     => Ok(Some(Value::Reference(vm.new_class_object(  "java/lang/Integer".to_string())?))),
        "long"    => Ok(Some(Value::Reference(vm.new_class_object(     "java/lang/Long".to_string())?))),
        "short"   => Ok(Some(Value::Reference(vm.new_class_object(    "java/lang/Short".to_string())?))),
        "char"    => Ok(Some(Value::Reference(vm.new_class_object("java/lang/Character".to_string())?))),
        "byte"    => Ok(Some(Value::Reference(vm.new_class_object(     "java/lang/Byte".to_string())?))),
        "float"   => Ok(Some(Value::Reference(vm.new_class_object(    "java/lang/Float".to_string())?))),
        "double"  => Ok(Some(Value::Reference(vm.new_class_object(   "java/lang/Double".to_string())?))),
        "boolean" => Ok(Some(Value::Reference(vm.new_class_object(  "java/lang/Boolean".to_string())?))),
        "void"    => Ok(Some(Value::Reference(vm.new_class_object(     "java/lang/Void".to_string())?))),
        _ => Err(VmError::ValidationError(format!("Expected extractable string")))
    }
}

fn delegate_get_component_type<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("getComponentType \n'{:?}'\n'{:?}'", class_object, args);
    let class_name = vm.extract_string_from_object(&class_object.unwrap().get_field(5))?;
    //let field_type = field_type_from_str(class_name.as_str());
    debug!("getComponentType '{:?}'", class_name);

    let array_class = vm.get_or_resolve_class(class_name.as_str())?;
    if let Some(array_info) = &array_class.array_info{
        let component_class_object = vm.new_class_object(array_info.component_type.to_class_name())?;
        Ok(Some(Value::Reference(component_class_object)))
    } else {
        Err(VmError::ValidationError(format!("Expected Array object but found '{:?}'", class_object)))
    }
}

fn delegate_get_classloader<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("getClassLoader0");
    Ok(Some(Value::Null))
}

fn delegate_desired_assertion_status<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("desiredAssertionStatus0");
    Ok(Some(Value::Integer(1)))
}

fn delegate_get_declared_fields0<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("getDeclaredFields");
    if let Some(obj) = class_object {
        let class_name = vm.extract_string_from_object(&obj.get_field(5))?;
        debug!("class name: {}", class_name);
        let mut content = Vec::new();
        for field in vm.get_or_resolve_class(class_name.as_str())?.fields.iter(){
            let java_field = vm.new_object("java/lang/reflect/Field")?;
            //name
            java_field.set_field(6, Value::Reference(vm.new_string_object(field.name.clone())?));
            debug!("field name: {}", field.name);
            content.push(Value::Reference(java_field));
        }
        Ok(Some(Value::Reference(vm.new_array(1, FieldType::Object("java/lang/reflect/Field".to_string()), RefCell::new(content))?)))
    } else {
        Ok(None)
    }
}

fn delegate_get_declared_constructors0<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("getDeclaredConstructors");
    if let Some(class_ref) = class_object{
        let class = vm.extract_class_from_class_object(class_ref)?;
        let mut content = Vec::new();
        for constructor in class.get_constructors().iter(){
            let java_constructor = vm.new_object("java/lang/reflect/Constructor")?;

            //clazz
            java_constructor.set_field(4, Value::Reference(class_ref));

            let mut parameters = Vec::new();
            for field_type in constructor.descriptor.args.iter(){
                let parameter_class = vm.new_class_object(field_type.to_class_name())?;
                parameters.push(Value::Reference(parameter_class));
            }
            //parameterTypes
            java_constructor.set_field(6, Value::Reference(vm.new_array(1, FieldType::Object("java/lang/Class".to_string()), RefCell::new(parameters))?));

            let flags = constructor.flags.iter().map(|flag| flag.clone() as u16).reduce(|flag1, flag2| flag1 | flag2).unwrap_or(0);
            //modifiers
            java_constructor.set_field(8, Value::Integer(flags as i32));

            content.push(Value::Reference(java_constructor));
        }
        Ok(Some(Value::Reference(vm.new_array(1, FieldType::Object("java/lang/reflect/Constructor".to_string()), RefCell::new(content))?)))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_get_class_modifiers<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(obj) = class_object{
        let class = vm.extract_class_from_class_object(obj)?;
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        Ok(Some(Value::Integer(flags)))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_get_super_class<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, class_object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(obj) = class_object{
        let class = vm.extract_class_from_class_object(obj)?;
        match class.superclass {
            Some(super_class) => {
                let super_class_object = vm.new_class_object(super_class.name.clone())?;
                Ok(Some(Value::Reference(super_class_object)))
            }
            None => Ok(Some(Value::Null))
        }

    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_for_name0<'a>(vm: &mut VM<'a>,  _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("forName0");
    if let Some(name) = args.get(0) {
        let name = vm.extract_string_from_object(&name)?;
        let name = name.replace(".", "/");
        //let class = vm.find_class_by_name(name)?;
        Ok(Some(Value::Reference(vm.new_class_object(name)?)))
    } else {
        Err(VmError::ValidationError("no name".to_string()))
    }
}

fn delegate_is_interface<'a>(vm: &mut VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("isInterface {:?}", obj);
    if let Some(obj) = obj {
        let class = vm.extract_class_from_class_object(obj)?;
        Ok(Some(Value::Integer(if class.is_interface() { 1 } else { 0 })))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_array<'a>(vm: &mut VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("isArray {:?}", obj);
    if let Some(obj) = obj {
        let name_object = obj.get_field(5);
        let name = vm.extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        Ok(Some(Value::Integer(if name.starts_with("[") { 1 } else { 0 })))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_is_primitive<'a>(vm: &mut VM<'a>,  _: ClassRef<'a>, obj: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("isPrimitive {:?}", obj);
    if let Some(obj) = obj {
        let name_object = obj.get_field(5);
        let name = vm.extract_string_from_object(&name_object)?;
        Ok(Some(Value::Integer(match name.as_str() {
            "java/lang/Boolean" | "java/lang/Character" | "java/lang/Byte"  | "java/lang/Short"  |
            "java/lang/Integer" | "java/lang/Long"      | "java/lang/Float" | "java/lang/Double" |
            "java/lang/Void" => 1,
            _ => 0,
        })))
        //Ok(Some(Value::Integer(if PrimitiveType::from_str(name.as_str()).is_ok() { 1 } else { 0 })))
    } else {
        Err(VmError::ValidationError("this is Null".to_string()))
    }
}

fn delegate_float_to_raw_bits<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Float(value)) = args.get(0){
        return Ok(Some(Value::Integer(value.to_bits() as i32)))
    }
    Err(VmError::ValidationError(format!("Expected float")))
}

fn delegate_double_to_raw_bits<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Double(value)) = args.get(0){
        return Ok(Some(Value::Long(value.to_bits() as i64)))
    }
    Err(VmError::ValidationError(format!("Expected double")))
}

fn delegate_get_class<'a>(vm: &mut VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("getClass");
    debug!("{}", class.name);
    Ok(Some(Value::Reference(vm.new_class_object(class.name.clone())?)))
}

fn delegate_hashcode<'a>(_: &mut VM<'a>, _: ClassRef<'a>, reference: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //FIXME hash string not address
    if let Some(obj) = reference{
        let addr = &obj as *const _;
        let addr = addr as i32;
        println!("HASHCODE: {addr}");
        Ok(Some(Value::Integer(addr)))
    } else {
        Err(VmError::ValidationError("Expected object".to_string()))
    }
}

fn delegate_fill_in_stacktrace<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(receiver) = object{
        return Ok(Some(Value::Reference(receiver)));
    }
    return Err(VmError::ValidationError("Expected a Throwable".to_string()));
}

fn delegate_array_base_offset<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Reference(class)) = args.get(0){
        Ok(Some(Value::Integer(16)))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_array_index_scale<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Reference(class)) = args.get(0){
        Ok(Some(Value::Integer(1)))
    } else {
        Err(VmError::ValidationError("Expected a class object reference".to_string()))
    }
}

fn delegate_address_size<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(Value::Integer(8)))
}

fn delegate_object_field_offset<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //FIXME calc real offset
    Ok(Some(Value::Long(0)))
}

fn delegate_compare_and_swap_object<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(Value::Integer(1)))
}

fn delegate_compare_and_swap_int<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(Value::Integer(1)))
}

fn delegate_allocate_memory<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Long(num)) = args.get(0){
        //return is address in memory
        let ptr = vm.unsafe_allocator.allocate_memory(*num as usize);
        Ok(Some(Value::Long(ptr)))
    } else {
        Err(VmError::ValidationError("Expected a long".to_string()))
    }
}

fn delegate_put_long<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //because args = [Long, Dummy, Long, Dummy]
    if let (Some(Value::Long(ptr)), Some(Value::Long(value))) = (args.get(0), args.get(2)){
        vm.unsafe_allocator.put_long(*ptr, *value);
        Ok(None)
    } else {
        Err(VmError::ValidationError("Expected a long as address and a long as value".to_string()))
    }
}

fn delegate_get_byte<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Long(ptr)) = args.get(0){
        let byte = vm.unsafe_allocator.get_byte(*ptr);
        Ok(byte.map(|byte| Value::Integer(byte as i32)))
    } else {
        Err(VmError::ValidationError("Expected a long as address".to_string()))
    }
}

fn delegate_get_caller_class<'a>(vm: &mut VM<'a>, class : ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(Value::Reference(vm.new_class_object(class.name.clone())?)))
}

fn delegate_get_class_access_flags<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Reference(obj)) = args.get(0){
        let class = vm.extract_class_from_class_object(obj)?;
        let flags = class.flags.iter().cloned().map(|val| val as u16).reduce(|val1, val2| val1 | val2).unwrap_or(0) as i32;
        Ok(Some(Value::Integer(flags)))
    } else {
        Err(VmError::ValidationError("Expected Class object".to_string()))
    }
}

fn delegate_current_thread<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if vm.current_thread.is_none(){
        let thread = vm.new_object("java/lang/Thread")?;
        //let thread_init = vm.resolve_class_method("java/lang/Thread", "<init>", "()V")?;
        //vm.invoke(thread_init, Some(thread), vec![])?;
        let name_string = vm.new_string_object("Main".to_string())?;
        let name_char_array = name_string.get_field(0);

        let group = vm.new_object("java/lang/ThreadGroup")?;
        let group_init = vm.resolve_class_method("java/lang/ThreadGroup", "<init>", "()V")?;
        vm.invoke(group_init, Some(group), vec![])?;

        thread.set_field(0, name_char_array);
        thread.set_field(1, Value::Integer(10));
        thread.set_field(8, Value::Reference(group));
        vm.current_thread = Some(thread);
        Ok(Some(Value::Reference(thread)))
    } else {
        Ok(Some(Value::Reference(vm.current_thread.unwrap())))
    }
}

fn delegate_is_alive<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(object.unwrap().get_field(5)))
}

fn delegate_get_stack_access_control_context<'a>(_: &mut VM<'a>, _: ClassRef<'a>, _: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    Ok(Some(Value::Null))
}

fn delegate_do_privileged<'a>(vm: &mut VM<'a>, class: ClassRef<'a>, _: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Reference(action)) = args.get(0){
        let class_name = vm.find_class_by_id(action.class_id).unwrap().name.as_str();
        let run = vm.resolve_class_method(class_name, "run", "()Ljava/lang/Object;")?;
        Ok(vm.invoke(run, Some(action), vec![])?)
    } else {
        Err(VmError::ValidationError("Expected a action object reference".to_string()))
    }
}

fn delegate_string_intern<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(obj) = object{
        let content = vm.extract_string_from_object(&Value::Reference(obj))?;
        warn!("String {} exists already? '{}'", content, vm.string_objects.contains_key(&content));
        if vm.string_objects.contains_key(&content){
            Ok(Some(Value::Reference(vm.string_objects[&content])))
        } else {
            Ok(Some(Value::Reference(obj)))
        }
    } else {
        Err(VmError::ValidationError("Expected a string object reference".to_string()))
    }
}

fn delegate_new_instance0<'a>(vm: &mut VM<'a>, _: ClassRef<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    debug!("newInstance0");
    debug!("{:?}", args);
    if let Some(Value::Reference(constructor)) = args.get(0){
        let clazz = constructor.get_field(4);
        let parameter_types = constructor.get_field(6);
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
                    let object = vm.new_object(class_and_method.class.name.as_str())?;
                    vm.invoke(class_and_method, Some(object), constructor_args)?;
                    return Ok(Some(Value::Reference(object)))
                }
            }
        }
        Ok(None)
    } else {
        Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
    }
    /*if let (Some(Value::Reference(constructor)), Some(Value::Reference(argument_array))) = (args.get(0), args.get(1)){
        if let ReferenceType::Array(_, _, args_content) = &argument_array.reference_type{
            //TODO add something like Value.expect_reference
            let class = constructor.get_field(4);
            let parameter_types = constructor.get_field(6);
            if let (Value::Reference(class_ref), Value::Reference(parameter_array)) = (class, parameter_types){
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

                }
            }

            Ok(None)
        } else {
            Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
        }
    } else {
        Err(VmError::ValidationError("Expected a constructor object and a array reference".to_string()))
    }*/
}