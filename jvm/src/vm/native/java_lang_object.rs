use crate::vm::constants::classes::{JAVA_LANG_OBJECT, JAVA_LANG_OBJECT_ARR};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VmError, VM};
use log::{debug, trace};
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate| registry.register(JAVA_LANG_OBJECT, method_name, sig, delegate);
    register("getClass", "()Ljava/lang/Class;", delegate_get_class);
    register("hashCode", "()I", delegate_hashcode);
    register("clone", "()Ljava/lang/Object;", delegate_clone);
    registry.register(JAVA_LANG_OBJECT_ARR, "getClass", "()Ljava/lang/Class;", delegate_get_class);
    registry.register(JAVA_LANG_OBJECT_ARR, "clone", "()Ljava/lang/Object;", delegate_clone);
}

gen_delegate!(delegate_get_class, |vm, java_vm, obj_ref, _args| {
    //TODO check
    debug!("getClass");
    if let Some(obj_ref) = obj_ref {
        debug!("obj: {:?}", obj_ref.class_name);
        let class_ref = wrap_init!(vm, java_vm, vm.new_class_object_by_name(obj_ref.class_name.as_str())?);
        non_failing_some(Value::Reference(class_ref))
    } else {
        invalidation!("Object is Null")
    }
});

gen_delegate!(delegate_hashcode, |_vm, _java_vm, obj_ref, _args| {
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

gen_delegate!(delegate_clone, |vm, java_vm, obj_ref, _args| {
    debug!("clone");
    if let Some(obj_ref) = obj_ref{
        match &obj_ref.reference_type {
            ReferenceType::Array(dims, component_type, content) => {
                debug!("Cloning array: {:?}", obj_ref);
                let new_array_ref = wrap_init!(vm, java_vm, vm.new_array(*dims, component_type.clone().to_array_field_type(*dims), content.clone())?);
                vm.debug_helper.tracker.push_object_event(new_array_ref.id, format!("Cloned from:\n    {:?}", obj_ref));
                non_failing_some(Value::Reference(new_array_ref))
            }
            ReferenceType::Object(content) => {
                debug!("Cloning object: {:?}", obj_ref);
                let new_object_ref = wrap_init!(vm, java_vm, vm.new_object(obj_ref.class_name.as_str())?);
                vm.debug_helper.tracker.push_object_event(new_object_ref.id, format!("Cloned from:\n    {:?}", obj_ref));
                if let ReferenceType::Object(new_content) = &new_object_ref.reference_type{
                    let _ = new_content.replace(content.borrow().clone());
                }
                non_failing_some(Value::Reference(new_object_ref))
            }
        }
    } else {
        invalidation!("Expected object")
    }
});