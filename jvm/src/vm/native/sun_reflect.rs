use crate::class_file::fields::get_class_descriptor;
use crate::vm::class::ClassAndMethod;
use crate::vm::constants::classes::{SUN_REFLECT_NCAI, SUN_REFLECT_NMAI, SUN_REFLECT_REFLECTION};
use crate::vm::constants::{CONSTRUCTOR_clazz_INDEX, CONSTRUCTOR_parameterTypes_INDEX, METHOD_clazz_INDEX, METHOD_name_INDEX, METHOD_parameterTypes_INDEX, METHOD_returnType_INDEX};
use crate::vm::java_error::JavaError;
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VmError, VM};
use log::debug;
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(SUN_REFLECT_REFLECTION, "getCallerClass", "()Ljava/lang/Class;", delegate_get_caller_class);
    registry.register(SUN_REFLECT_REFLECTION, "getClassAccessFlags", "(Ljava/lang/Class;)I", delegate_get_class_access_flags);
    registry.register(SUN_REFLECT_NCAI, "newInstance0", "(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_new_instance0);
    registry.register(SUN_REFLECT_NMAI, "invoke0", "(Ljava/lang/reflect/Method;Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;", delegate_invoke0);
}

gen_delegate!(delegate_get_caller_class, |ctx, _obj_ref, _args| {
    let frame_index = ctx.thread.call_stack.frames.borrow().len() - 2 - 1;
    if let Some(frame) = ctx.thread.call_stack.frames.borrow().get(frame_index){
        non_failing_some(Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object_by_name(frame.class_and_method.class.name.as_str())?)))
    } else {
        invalidation!("There is no parent Callframe")
    }
});

gen_delegate!(delegate_get_class_access_flags, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(class_ref)) = args.get(0){
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        let flags = clazz.flags as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_new_instance0, |ctx, _obj_ref, args| {
    debug!("newInstance0");
    debug!("{:?}", args);
    if let Some(Value::Reference(constructor)) = args.get(0) {
        //clazz
        let clazz = constructor.get_field(CONSTRUCTOR_clazz_INDEX);
        //parameterTypes
        let parameter_types = constructor.get_field(CONSTRUCTOR_parameterTypes_INDEX);
        if let (Value::Reference(class_ref), Value::Reference(parameter_array)) = (clazz, parameter_types) {
            if let ReferenceType::Array(_, _, type_content) = &parameter_array.reference_type {
                let class = ctx.vm.extract_class_from_class_object(class_ref)?;
                let mut descriptor = String::from("(");
                for constructor_parameter_type in type_content.borrow().iter() {
                    if let Value::Reference(parameter_type_ref) = constructor_parameter_type {
                        let class = ctx.vm.extract_class_from_class_object(parameter_type_ref)?;
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
                    let constructor_args = if let Some(Value::Reference(argument_array)) = args.get(1) {
                        if let ReferenceType::Array(_, _, args_content) = &argument_array.reference_type{
                            args_content.borrow().clone()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };
                    // we have to do this manually because vm.new_object() tries to resolve and init the class
                    // the problem is that if the class is anonymous it can't be resolved and it crashes
                    wrap_init!(ctx, ctx.ensure_initialized(class)?);
                    let object = ctx.vm.new_object_from_class(class);
                    let res = JavaThread::invoke_subroutine(ctx, class_and_method, Some(object), constructor_args);
                    // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
                    // because it doesn't know whether it is a subroutine or not
                    return match res {
                        Ok(VMResultType::Successful(None)) => { non_failing_some(Value::Reference(object)) }
                        Ok(VMResultType::Successful(Some(value))) => { invalidation!("Constructor should not return anything: {:?}", value) }
                        Ok(typ) => unreachable!("{:?} can't escape invoke_frames_until", typ),
                        Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
                        Err(e) => Err(e),
                    }
                }
            }
        }
        unreachable!()
    } else {
        invalidation!("Expected a constructor object and a array reference")
    }
});

gen_delegate!(delegate_invoke0, |ctx, _obj_ref, args| {
    debug!("invoke0");
    debug!("{:?}", args);
    if let (Some(Value::Reference(method)), Some(Value::Reference(obj)), Some(Value::Reference(args_array_ref))) = (args.get(0), args.get(1), args.get(2)) {
        let class_val = method.get_field(METHOD_clazz_INDEX);
        let method_name_val = method.get_field(METHOD_name_INDEX);
        let return_type_val = method.get_field(METHOD_returnType_INDEX);
        let parameter_types = method.get_field(METHOD_parameterTypes_INDEX);
        if let (Value::Reference(class_ref), Value::Reference(return_type_ref), Value::Reference(parameter_array)) = (class_val, return_type_val, parameter_types) {
            if let ReferenceType::Array(_, _, type_content) = &parameter_array.reference_type {
                let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
                let mut descriptor = String::from("(");
                for method_parameter_type_val in type_content.borrow().iter() {
                    if let Value::Reference(parameter_type_ref) = method_parameter_type_val {
                        let class = ctx.vm.extract_class_from_class_object(parameter_type_ref)?;
                        if !class.is_array(){
                            descriptor.push_str(&get_class_descriptor(&class.name));
                        } else {
                            descriptor.push_str(&class.name);
                        }
                    }
                }
                descriptor.push_str(")");
                if !return_type_ref.is_null(){
                    let return_type = ctx.vm.extract_class_from_class_object(return_type_ref)?;
                    if !return_type.is_array(){
                        descriptor.push_str(&get_class_descriptor(&return_type.name));
                    } else {
                        descriptor.push_str(&return_type.name);
                    }
                }
                let method_name = VM::extract_string_from_object(&method_name_val)?;
                if let Some(method) = clazz.find_method(method_name.as_str(), descriptor.as_str()) {
                    debug!("method: {:?}", method);
                    let class_and_method = ClassAndMethod {class: clazz, method};
                    let method_args = if let ReferenceType::Array(_, _, args_content) = &args_array_ref.reference_type {
                        args_content.borrow().clone()
                    } else {
                        Vec::new()
                    };
                    let _clazz = wrap_init!(ctx, ctx.ensure_initialized(clazz)?);
                    let res = JavaThread::invoke_subroutine(ctx, class_and_method, if !obj.is_null() {Some(obj)} else {None}, method_args);
                    // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
                    // because it doesn't know whether it is a subroutine or not
                    return match res {
                        Ok(VMResultType::Successful(None)) => {
                            assert!(return_type_ref.is_null());
                            non_failing_some(ctx.vm.null())
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
    invalidation!("Expected a method object, this object and a array reference")
});