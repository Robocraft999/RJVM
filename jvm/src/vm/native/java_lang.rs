use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::constants::classes::{JAVA_LANG_PROCESS_ENVIRONMENT, JAVA_LANG_RUNTIME, JAVA_LANG_STRING, JAVA_LANG_THREAD, JAVA_LANG_THROWABLE};
use crate::vm::constants::{THREADGROUP_maxPriority_INDEX, THREADGROUP_nUnstartedThreads_INDEX, THREADGROUP_name_INDEX, THREADGROUP_parent_INDEX, THREAD_group_INDEX, THREAD_name_INDEX, THREAD_priority_INDEX};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult};
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};
use std::cell::RefCell;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_LANG_THROWABLE, "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    registry.register(JAVA_LANG_THROWABLE, "getStackTraceDepth", "()I", delegate_stack_trace_depth);
    registry.register(JAVA_LANG_STRING, "intern", "()Ljava/lang/String;", delegate_intern);
    registry.register(JAVA_LANG_THREAD, "currentThread", "()Ljava/lang/Thread;", delegate_current_thread);
    registry.register(JAVA_LANG_THREAD, "isAlive", "()Z", delegate_is_alive);
    registry.register(JAVA_LANG_RUNTIME, "availableProcessors", "()I", delegate_available_processors);
    registry.register(JAVA_LANG_RUNTIME, "freeMemory", "()J", delegate_free_memory);
    registry.register(JAVA_LANG_PROCESS_ENVIRONMENT, "environ", "()[[B", delegate_environ);
}

gen_delegate!(delegate_fill_in_stacktrace, |_vm, _java_vm, obj_ref, _args| {
    if let Some(obj_ref) = obj_ref{
        non_failing_some(Value::Reference(obj_ref))
    } else {
        invalidation!("Expected a Throwable")
    }
});

gen_delegate!(delegate_stack_trace_depth, |_vm, _java_vm, obj_ref, _args| {
    if let Some(_obj_ref) = obj_ref{
        non_failing_some(Value::Integer(0))
    } else {
        invalidation!("Expected a Throwable")
    }
});

gen_delegate!(delegate_intern, |vm, _java_vm, obj_ref, _args| {
    if let Some(obj) = obj_ref{
        let content = VM::extract_string_from_object(&Value::Reference(obj))?;
        if vm.string_objects.borrow().contains_key(&content){
            non_failing_some(Value::Reference(vm.string_objects.borrow()[&content]))
        } else {
            non_failing_some(Value::Reference(obj))
        }
    } else {
        invalidation!("Expected a string object reference")
    }
});

gen_delegate!(delegate_current_thread, |vm, java_vm, _obj_ref, _args| {
    if vm.current_thread.borrow().is_none(){
        let thread = wrap_init!(vm, java_vm, vm.new_object(JAVA_LANG_THREAD)?);
        //let thread_init = vm.resolve_class_method("java/lang/Thread", "<init>", "()V")?;
        //vm.invoke(thread_init, Some(thread), vec![])?;
        let name_string = wrap_init!(vm, java_vm, vm.new_string_object("Main")?);

        // TODO call the private contructor directly
        let group_name = wrap_init!(vm, java_vm, vm.new_string_object("system")?);
        let group = wrap_init!(vm, java_vm, vm.new_object("java/lang/ThreadGroup")?);
        group.set_field(THREADGROUP_nUnstartedThreads_INDEX, Value::Integer(0));
        group.set_field(THREADGROUP_name_INDEX, Value::Reference(group_name));
        group.set_field(THREADGROUP_maxPriority_INDEX, Value::Integer(10));
        group.set_field(THREADGROUP_parent_INDEX, vm.null());

        //let group_init = vm.try_resolve_class_method("java/lang/ThreadGroup", "<init>", "()V")?;
        //vm.invoke_new_frame(group_init, Some(group), vec![])?;

        thread.set_field(THREAD_name_INDEX, Value::Reference(name_string));
        thread.set_field(THREAD_priority_INDEX, Value::Integer(10));
        thread.set_field(THREAD_group_INDEX, Value::Reference(group));
        let _ = vm.current_thread.replace(Some(thread));
        non_failing_some(Value::Reference(thread))
    } else {
        non_failing_some(Value::Reference(vm.current_thread.borrow().unwrap()))
    }
});

gen_delegate!(delegate_is_alive, |_vm, _java_vm, obj_ref, _args| {
    //non_failing_some(Value::Integer(1))
    // FIXME threading
    non_failing_some(obj_ref.unwrap().get_field(5))
});

gen_delegate!(delegate_available_processors, |_vm, _java_vm, _obj_ref, _args| {
    non_failing_some(Value::Integer(1))
});

gen_delegate!(delegate_free_memory, |_vm, _java_vm, _obj_ref, _args| {
    non_failing_some(Value::Long(1024 * 1024 * 20))
});

gen_delegate!(delegate_environ, |vm, java_vm, _obj_ref, _args| {
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
});