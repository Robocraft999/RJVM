use crate::vm::java_error::JavaError;
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_some, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register("java/security/AccessController", "getStackAccessControlContext", "()Ljava/security/AccessControlContext;", delegate_get_stack_access_control_context);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/security/AccessController", "doPrivileged", "(Ljava/security/PrivilegedExceptionAction;Ljava/security/AccessControlContext;)Ljava/lang/Object;", delegate_do_privileged);
    registry.register("java/util/concurrent/atomic/AtomicLong", "VMSupportsCS8", "()Z", delegate_vm_supports_cs8);
}


gen_delegate!(delegate_get_stack_access_control_context, |ctx, _obj_ref, _args| {
    non_failing_some(ctx.vm.null())
});

gen_delegate!(delegate_do_privileged, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(action)) = args.get(0) {
        let clazz = ctx.vm.find_class_by_id(action.class_id).unwrap();
        // FIXME clazz.find_method should be sufficient here
        let run = clazz.resolve_method_virtual("run", "()Ljava/lang/Object;").unwrap();
        let res = JavaThread::invoke_subroutine(ctx, run, Some(action), vec![]);

        // invoke_frames_until returns occurred exceptions as Err(VmError::JavaException(JavaError::JavaExceptionThrown))
        // because it doesn't know whether it is a subroutine or not
        match res{
            Ok(any) => Ok(any),
            Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
            Err(e) => Err(e),
        }
    } else {
        invalidation!("Expected a action object reference")
    }
});

gen_delegate!(delegate_vm_supports_cs8, |_ctx, _obj_ref, _args| {
    non_failing_some(Value::Integer(0))
});