use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::application::{thread, JAVA_THREAD};
use crate::vm::class::ClassAndMethod;
use crate::vm::constants::classes::{JAVA_LANG_PACKAGE, JAVA_LANG_PROCESS_ENVIRONMENT, JAVA_LANG_REFLECT_ARRAY, JAVA_LANG_RUNTIME, JAVA_LANG_STRING, JAVA_LANG_THREAD, JAVA_LANG_THROWABLE};
use crate::vm::constants::{THREADGROUP_maxPriority_INDEX, THREADGROUP_nUnstartedThreads_INDEX, THREADGROUP_name_INDEX, THREADGROUP_parent_INDEX, THREAD_daemon_INDEX, THREAD_group_INDEX, THREAD_name_INDEX, THREAD_priority_INDEX, THREAD_target_INDEX};
use crate::vm::java_thread::JavaThread;
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult};
use crate::vm::value::{Reference, Value};
use crate::vm::{jni, Context, VmError, VM};
use log::error;
use parking_lot::RwLock;
use std::thread;
use std::thread::yield_now;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_LANG_THROWABLE, "fillInStackTrace", "(I)Ljava/lang/Throwable;", delegate_fill_in_stacktrace);
    registry.register(JAVA_LANG_THROWABLE, "getStackTraceDepth", "()I", delegate_stack_trace_depth);
    registry.register(JAVA_LANG_STRING, "intern", "()Ljava/lang/String;", delegate_intern);
    registry.register(JAVA_LANG_THREAD, "currentThread", "()Ljava/lang/Thread;", delegate_current_thread);
    registry.register(JAVA_LANG_THREAD, "isAlive", "()Z", delegate_is_alive);
    registry.register(JAVA_LANG_THREAD, "holdsLock", "(Ljava/lang/Object;)Z", delegate_holds_lock);
    registry.register(JAVA_LANG_THREAD, "setPriority0", "(I)V", delegate_set_priority0);
    registry.register(JAVA_LANG_THREAD, "start0", "()V", delegate_start0);
    registry.register(JAVA_LANG_THREAD, "yield", "()V", delegate_yield);
    registry.register(JAVA_LANG_THREAD, "isInterrupted", "(Z)Z", delegate_is_interrupted);
    registry.register(JAVA_LANG_RUNTIME, "availableProcessors", "()I", delegate_available_processors);
    registry.register(JAVA_LANG_RUNTIME, "freeMemory", "()J", delegate_free_memory);
    registry.register(JAVA_LANG_PROCESS_ENVIRONMENT, "environ", "()[[B", delegate_environ);
    registry.register(JAVA_LANG_PACKAGE, "getSystemPackage0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_get_system_package0);
    registry.register(JAVA_LANG_REFLECT_ARRAY, "newArray", "(Ljava/lang/Class;I)Ljava/lang/Object;", delegate_new_array);
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
        if ctx.vm.string_objects.read().contains_key(&content){
            non_failing_some(Value::Reference(ctx.vm.string_objects.read()[&content].id))
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
    non_failing_some(Value::from(ctx.vm.monitor_handler.holds_lock(ctx, *lock_ref)))
});

gen_delegate!(delegate_set_priority0, |_ctx, obj_ref, args| {
    let Some(thread_ref) = obj_ref else { return invalidation!("setPriority0 expected a Thread reference"); };
    let Some(Value::Integer(java_prio)) = args.get(0) else { return invalidation!("setPriority0 expected a prio arg"); };
    thread_ref.set_field(THREAD_priority_INDEX, Value::Integer(*java_prio));
    non_failing_none()
});

gen_delegate!(delegate_start0, |ctx, obj_ref, _args| {
    let Some(obj_ref) = obj_ref else { return invalidation!("Expected this to be present") };
    let obj_id = obj_ref.id;
    let thread_name = ctx.vm.extract_string_from_value(obj_ref.get_field(THREAD_name_INDEX))?;
    let target_id = obj_ref.get_ref_field(THREAD_target_INDEX)?;
    let target = if target_id.is_null() {
        obj_ref
    } else {
        ctx.vm.resolve_object_by_id(target_id)?
    };
    let is_daemon = obj_ref.get_int_field(THREAD_daemon_INDEX)? == 1;

    let target_clazz = ctx.vm.find_class_by_id(target.class_id).unwrap();
    let method = target_clazz.find_method("run", "()V").unwrap();
    let cam = ClassAndMethod { class: target_clazz, method };
    let camid = cam.as_ids();
    let target_id = target.id;

    let id = {
        let mut next_id = ctx.vm.next_thread_id.lock();
        let current = *next_id;
        *next_id += 1;
        current
    };
    let vm_ptr = ctx.vm as *const VM as _;
    let env = Box::pin(JNIEnv::new(vm_ptr));
    let vm = unsafe { &*(vm_ptr as *const VM)};
    let java_vm = Box::pin(JavaVM::new());
    
    thread::Builder::new().name(format!("T<{}>({})", id, thread_name)).spawn(move || {
        let mut java_thread = JavaThread::new(id, is_daemon);
        java_thread.thread_obj_id.replace(obj_id);

        java_thread.jni_env = env;
        java_thread.java_vm = java_vm;

        vm.thread_lookup.write().insert(obj_id, java_thread.meta.clone());
    
        JAVA_THREAD.set(java_thread);

        let context = Context { thread: thread(), vm};

        let result = JavaThread::thread_entry(context, camid, target_id, Vec::new());
        if let Err(err) = result {
            error!("Thread failed with: {}", err);
            context.vm.mark_canceled();
        }
        context.thread.meta.finish();
    }).unwrap();
    non_failing_none()
});

gen_delegate!(delegate_yield, |_ctx, _obj_ref, _args| {
    yield_now();
    non_failing_none()
});

gen_delegate!(delegate_is_interrupted, |ctx, _obj_ref, args| {
    let Some(Value::Integer(clear_interrupted)) = args.get(0) else { return invalidation!("Expected a boolean parameter") };

    let was_interrupted = ctx.thread.meta.interrupted.read().clone();

    if *clear_interrupted == 1 {
        *ctx.thread.meta.interrupted.write() = false;
    }

    non_failing_some(Value::from(was_interrupted))
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
    fn byte_array_from_str<'s>(ctx: &Context<'s, '_>, string: &str) -> VMResult<Reference<'s>>{
        ctx.try_new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RwLock::new(string.as_bytes().iter().map(|c| Value::Integer(*c as i32)).collect()))
    }
    let _ = wrap_init!(ctx, ctx.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RwLock::new(Vec::new()))?);
    let values: Vec<Value> = vars.iter()
        .flat_map(|(k, v)| vec![
            Value::Reference(byte_array_from_str(&ctx, k).unwrap().id),
            Value::Reference(byte_array_from_str(&ctx, v).unwrap().id),
        ])
        .collect();
    let array_ref = wrap_init!(ctx, ctx.new_array(2, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(2), RwLock::new(values.clone()))?);
    non_failing_some(Value::Reference(array_ref.id))
});

// return the path to the defining system jar like rt.jar or null if not system package
gen_delegate!(delegate_get_system_package0, |ctx, _obj_ref, args| {
    if let Some(name_val) = args.get(0) {
        let name = ctx.vm.extract_string_from_value(*name_val)?;
        let (prefix, _) = name.rsplit_once("/").unwrap();
        non_failing_some(ctx.vm.null())
    } else {
        invalidation!("Expected a string argument")
    }
});

gen_delegate!(delegate_new_array, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(class_ref_id)), Some(Value::Integer(length))) = (args.get(0), args.get(1)) {
        let clazz = ctx.resolve_clazz_by_class_ref_id(*class_ref_id)?;
        let content = vec![ctx.vm.null(); *length as usize];
        let arr_ref = wrap_init!(ctx, ctx.new_array(
            1,
            FieldType::Object(clazz.name.clone()).to_array_field_type(1),
            RwLock::new(content.clone())
        )?);
        non_failing_some(Value::Reference(arr_ref.id))
    } else {
        invalidation!("Expected class and int")
    }
});