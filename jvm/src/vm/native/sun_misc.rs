use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::constants::classes::{SUN_MISC_PERF, SUN_MISC_SIGNAL, SUN_MISC_URL_CLASSPATH, SUN_MISC_VM, SUN_NIO_FS_UND};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};
use log::debug;
use std::cell::RefCell;
use std::env;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(SUN_MISC_VM, "initialize", "()V", delegate_initialize);
    registry.register(SUN_MISC_SIGNAL, "findSignal", "(Ljava/lang/String;)I", delegate_find_signal);
    registry.register(SUN_MISC_SIGNAL, "handle0", "(IJ)J", delegate_handle0);
    registry.register(SUN_MISC_URL_CLASSPATH, "getLookupCacheURLs", "(Ljava/lang/ClassLoader;)[Ljava/net/URL;", delegate_lookup_cache_urls);
    registry.register(SUN_MISC_PERF, "createLong", "(Ljava/lang/String;IIJ)Ljava/nio/ByteBuffer;", delegate_create_long);
    registry.register(SUN_NIO_FS_UND, "init", "()I", delegate_und_init);
    registry.register(SUN_NIO_FS_UND, "getcwd", "()[B", delegate_und_getcwd);
}

gen_delegate!(delegate_initialize, |vm, _java_vm, _obj_ref, _args| {
    let _vm_class_id = vm.find_class_by_name(SUN_MISC_VM).unwrap().id;
    /*let arg1 = wrap_init!(vm, java_vm, vm.new_string_object("java.lang.Integer.IntegerCache.high".to_string())?);
    let arg2 = wrap_init!(vm, java_vm, vm.new_string_object("127".to_string())?);
    let static_vm_object = vm.get_static_class_object(vm_class_id).unwrap();
    let properties_object = static_vm_object.get_field(11).expect_reference()?;

    let save_properties_method = vm.try_resolve_class_method("sun/misc/VM", "saveAndRemoveProperties", "(Ljava/util/Properties;)V")?;
    let frame2 = vm.call_stack.create_and_push_call_frame(save_properties_method, None, vec![Value::Reference(properties_object)], false);
    let properties_set_method = vm.try_resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let frame1 = vm.call_stack.create_and_push_call_frame(properties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)], false);*/
    //Ok(VMResultType::NeedsClassInit(vec![(), ()], false))
    non_failing_none()
});


gen_delegate!(delegate_find_signal, |_vm, _java_vm, _obj_ref, args| {
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
});

gen_delegate!(delegate_handle0, |_vm, _java_vm, _obj_ref, _args| {
    non_failing_some(Value::Long(0))
});

gen_delegate!(delegate_lookup_cache_urls, |vm, _java_vm, _obj_ref, _args| {
    //FIXME add cache, idk how to get this
    non_failing_some(vm.null())
});

gen_delegate!(delegate_create_long, |vm, java_vm, _obj_ref, _args| {
    let class_name = "java/nio/DirectByteBuffer";
    let byte_buffer = wrap_init!(vm, java_vm, vm.new_object(class_name)?);
    let constructor = vm.resolve_class_method(class_name, "<init>", "(JI)V")?;
    let frame_index = vm.call_stack.frames.borrow().len() as isize - 1;
    let addr = vm.unsafe_allocator.allocate_memory(8);
    vm.call_stack.create_and_push_call_frame(constructor, Some(byte_buffer), vec![Value::Long(addr), Value::Dummy, Value::Integer(8)], false);
    let res = vm.invoke_frames_until(java_vm, frame_index)?;
    if let VMResultType::Successful(None) = res{
        non_failing_some(Value::Reference(byte_buffer))
    } else {
        invalidation!("Error when calling constructor")
    }
});

gen_delegate!(delegate_und_init, |_vm, _java_vm, _obj_ref, _args| {
    non_failing_some(Value::Integer(0))
});

gen_delegate!(delegate_und_getcwd, |vm, java_vm, _obj_ref, _args| {
    let current_working_dir = env::current_dir().unwrap();
    debug!("getcwd -> '{}'", current_working_dir.display());
    let bytes = current_working_dir.into_os_string().as_encoded_bytes().iter().map(|b| Value::Integer(*b as i32)).collect::<Vec<_>>();
    let path_ref = wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1), RefCell::new(bytes.clone()))?);
    non_failing_some(Value::Reference(path_ref))
});