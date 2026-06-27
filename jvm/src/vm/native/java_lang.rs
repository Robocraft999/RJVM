use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::constants::classes::{JAVA_LANG_PROCESS_ENVIRONMENT, JAVA_LANG_RUNTIME, JAVA_LANG_STRING, JAVA_LANG_THREAD, JAVA_LANG_THROWABLE};
use crate::vm::constants::{THREADGROUP_maxPriority_INDEX, THREADGROUP_nUnstartedThreads_INDEX, THREADGROUP_name_INDEX, THREADGROUP_parent_INDEX, THREAD_group_INDEX, THREAD_name_INDEX, THREAD_priority_INDEX};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult};
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};
use std::cell::RefCell;
use std::thread;
use crate::vm::application::JAVA_THREAD;
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_LANG_THROWABLE, "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    registry.register(JAVA_LANG_THROWABLE, "getStackTraceDepth", "()I", delegate_stack_trace_depth);
    registry.register(JAVA_LANG_STRING, "intern", "()Ljava/lang/String;", delegate_intern);
    registry.register(JAVA_LANG_THREAD, "currentThread", "()Ljava/lang/Thread;", delegate_current_thread);
    registry.register(JAVA_LANG_THREAD, "isAlive", "()Z", delegate_is_alive);
    registry.register(JAVA_LANG_THREAD, "holdsLock", "(Ljava/lang/Object;)Z", delegate_holds_lock);
    registry.register(JAVA_LANG_THREAD, "start0", "()V", delegate_start0);
    registry.register(JAVA_LANG_RUNTIME, "availableProcessors", "()I", delegate_available_processors);
    registry.register(JAVA_LANG_RUNTIME, "freeMemory", "()J", delegate_free_memory);
    registry.register(JAVA_LANG_PROCESS_ENVIRONMENT, "environ", "()[[B", delegate_environ);
}

gen_delegate!(delegate_fill_in_stacktrace, |_ctx, obj_ref, _args| {
    if let Some(obj_ref) = obj_ref{
        non_failing_some(Value::Reference(obj_ref.id))
    } else {
        invalidation!("Expected a Throwable")
    }
});

gen_delegate!(delegate_stack_trace_depth, |_ctx, obj_ref, _args| {
    if let Some(_obj_ref) = obj_ref{
        non_failing_some(Value::Integer(0))
    } else {
        invalidation!("Expected a Throwable")
    }
});

gen_delegate!(delegate_intern, |ctx, obj_ref, _args| {
    if let Some(obj) = obj_ref{
        let content = ctx.vm.extract_string_from_ref(obj)?;
        if ctx.vm.string_objects.read()?.contains_key(&content){
            non_failing_some(Value::Reference(ctx.vm.string_objects.read()?[&content].id))
        } else {
            non_failing_some(Value::Reference(obj.id))
        }
    } else {
        invalidation!("Expected a string object reference")
    }
});

gen_delegate!(delegate_current_thread, |ctx, _obj_ref, _args| {
    let Some(thread_ref_id) = ctx.thread.thread_obj_id else { return invalidation!("Thread object id has to be set by now") };
    non_failing_some(Value::Reference(thread_ref_id))
});

gen_delegate!(delegate_is_alive, |_ctx, obj_ref, _args| {
    //non_failing_some(Value::Integer(1))
    // FIXME threading
    non_failing_some(obj_ref.unwrap().get_field(5))
});

gen_delegate!(delegate_holds_lock, |ctx, _obj_ref, args| {
    let Some(Value::Reference(lock_ref)) = args.get(0) else { return invalidation!("holdLock expected a potential lock"); };
    //let current_thread = ctx.vm.current_thread.borrow();
    //let Some(_current_thread) = current_thread.as_ref() else { return invalidation!("There is no thread lol"); };
    non_failing_some(Value::from(ctx.vm.current_locks.read()?.contains_key(&lock_ref)))
});

gen_delegate!(delegate_start0, |ctx, obj_ref, _args| {
   let Some(obj_ref) = obj_ref else { return invalidation!("Expected this to be present") };
    let obj_id = obj_ref.id;
    
    thread::spawn(move || {
        let mut java_thread = JavaThread::new(1);
        java_thread.thread_obj_id.replace(obj_id);
    
        JAVA_THREAD.set(java_thread);
    });
    non_failing_none()
});

gen_delegate!(delegate_available_processors, |_ctx, _obj_ref, _args| {
    non_failing_some(Value::Integer(1))
});

gen_delegate!(delegate_free_memory, |_ctx, _obj_ref, _args| {
    non_failing_some(Value::Long(1024 * 1024 * 20))
});

gen_delegate!(delegate_environ, |ctx, _obj_ref, _args| {
    let vars = vec![
        ("DISPLAY", ":0")
    ];
    fn byte_array_from_str<'s>(vm: &VM<'s>, string: &str) -> VMResult<Reference<'s>>{
        vm.try_new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(string.as_bytes().iter().map(|c| Value::Integer(*c as i32)).collect()))
    }
    let _ = wrap_init!(ctx, ctx.vm.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(Vec::new()))?);
    let values: Vec<Value> = vars.iter()
        .flat_map(|(k, v)| vec![
            Value::Reference(byte_array_from_str(ctx.vm, k).unwrap().id),
            Value::Reference(byte_array_from_str(ctx.vm, v).unwrap().id),
        ])
        .collect();
    let array_ref = wrap_init!(ctx, ctx.vm.new_array(2, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(2), RefCell::new(values.clone()))?);
    non_failing_some(Value::Reference(array_ref.id))
});