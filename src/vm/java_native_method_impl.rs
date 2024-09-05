use std::time::{SystemTime, UNIX_EPOCH};
use log::debug;
use crate::field_info::{FieldType, PrimitiveType};
use crate::method_info::MethodDescriptor;
use crate::vm::class::{ClassAndMethod, ClassRef};
use crate::vm::java_error::JavaError;
use crate::vm::value::Value;
use crate::vm::{VM, VmError};

use super::value::ObjectRef;

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

    pub fn invoke(vm: &mut VM<'a>, class_and_method: &ClassAndMethod<'a>, object: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Option<Result<Option<Value<'a>>, VmError>>{
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

type NativeMethodDelegate<'a> = fn(&mut VM<'a>, ClassRef<'a>, Option<ObjectRef<'a>>, Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>;

pub fn register_all_natives(registry: &mut NativeMethodRegistry){
    registry.register("java/lang/System", "nanoTime", "()J", delegate_nano_time);
    registry.register("java/lang/System", "currentTimeMillis", "()J", delegate_millis_time);
    registry.register("java/lang/System", "identityHashCode", "(Ljava/lang/Object;)I", delegate_identity_hash_code);
    registry.register("java/lang/System", "setOut0", "(Ljava/io/PrintStream;)V", delegate_set_out);
    registry.register("java/lang/Class", "getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_get_primitive_class);
    registry.register("java/lang/Class", "getClassLoader0", "()Ljava/lang/ClassLoader;", delegate_get_classloader);
    registry.register("java/lang/Class", "desiredAssertionStatus0", "(Ljava/lang/Class;)Z", delegate_desired_assertion_status);
    registry.register("java/lang/Float", "floatToRawIntBits", "(F)I", delegate_float_to_raw_bits);
    registry.register("java/lang/Double", "doubleToRawLongBits", "(D)J", delegate_double_to_raw_bits);
    registry.register("java/lang/Object", "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register("java/lang/Throwable", "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace)
}

fn delegate_nano_time<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    Ok(Some(Value::Long(nanos)))
}
fn delegate_millis_time<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    Ok(Some(Value::Long(millis)))
}

fn delegate_identity_hash_code<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Object(object)) = args.get(0){
        let addr = &object as *const _;
        let addr = addr as i32;
        println!("HASH: {addr}");
        Ok(Some(Value::Integer(addr)))
    } else {
        Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
    }
}

fn delegate_set_out<'a>(vm: &mut VM<'a>, class : ClassRef<'a>, _: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(static_object) = vm.get_static_class_object(class.id){
        if let Some(Value::Object(object)) = args.get(0){
            static_object.set_field(2, Value::Object(object));
            Ok(None)
        } else {
            Err(VmError::ValidationError(format!("Expected Object but found '{:?}'", args.get(0))))
        }
    } else {
        Err(VmError::ValidationError(format!("Couldn't find static Object of class {}", class.name)))
    }
}

fn delegate_get_primitive_class<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    let string = vm.extract_string_from_object(args.get(0).unwrap())?;
    match string.as_str() {
        "int"     => Ok(Some(Value::Object(vm.new_class_object(  "java/lang/Integer".to_string())?))),
        "long"    => Ok(Some(Value::Object(vm.new_class_object(     "java/lang/Long".to_string())?))),
        "short"   => Ok(Some(Value::Object(vm.new_class_object(    "java/lang/Short".to_string())?))),
        "char"    => Ok(Some(Value::Object(vm.new_class_object("java/lang/Character".to_string())?))),
        "byte"    => Ok(Some(Value::Object(vm.new_class_object(     "java/lang/Byte".to_string())?))),
        "float"   => Ok(Some(Value::Object(vm.new_class_object(    "java/lang/Float".to_string())?))),
        "double"  => Ok(Some(Value::Object(vm.new_class_object(   "java/lang/Double".to_string())?))),
        "boolean" => Ok(Some(Value::Object(vm.new_class_object(  "java/lang/Boolean".to_string())?))),
        _ => Err(VmError::ValidationError(format!("Expected extractable string")))
    }
}

fn delegate_get_classloader<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("getClassLoader0");
    Ok(Some(Value::Null))
}

fn delegate_desired_assertion_status<'a>(vm: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("desiredAssertionStatus0");
    Ok(Some(Value::Integer(1)))
}

fn delegate_float_to_raw_bits<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Float(value)) = args.get(0){
        return Ok(Some(Value::Integer(value.to_bits() as i32)))
    }
    Err(VmError::ValidationError(format!("Expected float")))
}

fn delegate_double_to_raw_bits<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, _: Option<ObjectRef<'a>>, args: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(Value::Double(value)) = args.get(0){
        return Ok(Some(Value::Long(value.to_bits() as i64)))
    }
    Err(VmError::ValidationError(format!("Expected float")))
}

fn delegate_get_class<'a>(vm: &mut VM<'a>, class : ClassRef<'a>, _: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    //TODO check
    debug!("getClass");
    debug!("{}", class.name);
    Ok(Some(Value::Object(vm.new_class_object(class.name.clone())?)))
}

fn delegate_fill_in_stacktrace<'a>(_: &mut VM<'a>, _ : ClassRef<'a>, object: Option<ObjectRef<'a>>, _: Vec<Value<'a>>) -> Result<Option<Value<'a>>, VmError>{
    if let Some(receiver) = object{
        return Ok(Some(Value::Object(receiver)));
    }
    return Err(VmError::ValidationError("Expected a Throwable".to_string()));
}