use crate::vm::constants::classes::{JAVA_LANG_OBJECT, JAVA_LANG_OBJECT_ARR};
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::VmError;
use log::{debug, trace};
use parking_lot::RwLock;
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate| registry.register(JAVA_LANG_OBJECT, method_name, sig, delegate);
    register("getClass", "()Ljava/lang/Class;", delegate_get_class);
    register("hashCode", "()I", delegate_hashcode);
    register("clone", "()Ljava/lang/Object;", delegate_clone);
    register("wait", "(J)V", delegate_wait);
    register("notify", "()V", delegate_notify);
    register("notifyAll", "()V", delegate_notify_all);
    registry.register(JAVA_LANG_OBJECT_ARR, "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register(JAVA_LANG_OBJECT_ARR, "clone", "()Ljava/lang/Object;", delegate_clone);
}

gen_delegate!(delegate_get_class, |ctx, obj_ref, _args| {
    //TODO check
    debug!("getClass");
    if let Some(obj_ref) = obj_ref {
        debug!("obj: {:?}", obj_ref.class_name);
        let class_ref = wrap_init!(ctx, ctx.new_class_object_by_name(obj_ref.class_name.as_str())?);
        non_failing_some(Value::Reference(class_ref.id))
    } else {
        invalidation!("Object is Null")
    }
});

gen_delegate!(delegate_hashcode, |_ctx, obj_ref, _args| {
    if let Some(obj_ref) = obj_ref{
        let mut hasher = DefaultHasher::new();
        obj_ref.id.hash(&mut hasher);
        let addr = hasher.finish() as i32;
        trace!(target: "native", "HASHCODE: {} {:?}", addr, obj_ref);
        non_failing_some(Value::Integer(addr))
    } else {
        invalidation!("Expected object")
    }
});

gen_delegate!(delegate_clone, |ctx, obj_ref, _args| {
    debug!("clone");
    if let Some(obj_ref) = obj_ref{
        match &obj_ref.reference_type {
            ReferenceType::Array(dims, component_type, content) => {
                debug!("Cloning array: {:?}", obj_ref);
                let new_array_ref = wrap_init!(ctx, ctx.new_array(*dims, component_type.clone().to_array_field_type(*dims), RwLock::new(content.read().clone()))?);
                ctx.thread.debug_helper.tracker.push_object_event(new_array_ref.id, format!("Cloned from:\n    {:?}", obj_ref));
                non_failing_some(Value::Reference(new_array_ref.id))
            }
            ReferenceType::Object(content) => {
                debug!("Cloning object: {:?}", obj_ref);
                let clazz = ctx.vm.find_class_by_id(obj_ref.class_id).unwrap();
                let new_object_ref = ctx.new_object_from_class(clazz);
                ctx.thread.debug_helper.tracker.push_object_event(new_object_ref.id, format!("Cloned from:\n    {:?}", obj_ref));
                if let ReferenceType::Object(new_content) = &new_object_ref.reference_type{
                    *new_content.write() = content.read().clone();
                }
                non_failing_some(Value::Reference(new_object_ref.id))
            }
        }
    } else {
        invalidation!("Expected object")
    }
});

gen_delegate!(delegate_wait, |ctx, obj_ref, args| {
    debug!("wait");
    if let (Some(obj_ref), Some(Value::Long(millies))) = (obj_ref, args.get(0)){
        if obj_ref.is_null() {
            return invalidation!("Cannot wait on null object");
        }
        ctx.vm.monitor_handler.wait(ctx, obj_ref.id, *millies as u64)?;
        non_failing_none()
    } else {
        invalidation!("Expected this and long param")
    }
});

gen_delegate!(delegate_notify, |ctx, obj_ref, _args| {
    debug!("notify");
    if let Some(obj_ref) = obj_ref{
        ctx.vm.monitor_handler.notify(ctx, obj_ref.id)?;
        non_failing_none()
    } else {
        invalidation!("Expected object")
    }
});

gen_delegate!(delegate_notify_all, |ctx, obj_ref, _args| {
    debug!("notify all");
    if let Some(obj_ref) = obj_ref{
        ctx.vm.monitor_handler.notify_all(ctx, obj_ref.id)?;
        non_failing_none()
    } else {
        invalidation!("Expected object")
    }
});