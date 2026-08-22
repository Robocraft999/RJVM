use crate::vm::constants::classes::JAVA_LANG_CLASSLOADER;
use crate::vm::constants::{CLASSLOADER_NATIVELIBRARY_handle_INDEX, CLASSLOADER_NATIVELIBRARY_loaded_INDEX, CLASSLOADER_NATIVELIBRARY_name_INDEX};
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::VmError;
use libloading::{Library, Symbol};
use log::debug;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate| registry.register(JAVA_LANG_CLASSLOADER, method_name, sig, delegate);
    register("findLoadedClass0", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_loaded_class0);
    register("findBootstrapClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_find_bootstrap_class);
    register("findBuiltinLib", "(Ljava/lang/String;)Ljava/lang/String;", delegate_find_builtin_lib);
    register("defineClass0", "(Ljava/lang/String;[BIILjava/security/ProtectionDomain;)Ljava/lang/Class;", delegate_define_class0);
    register("defineClass1", "(Ljava/lang/String;[BIILjava/security/ProtectionDomain;Ljava/lang/String;)Ljava/lang/Class;", delegate_define_class1);
    register("defineClass2", "(Ljava/lang/String;Ljava/nio/ByteBuffer;IILjava/security/ProtectionDomain;Ljava/lang/String;)Ljava/lang/Class;", delegate_define_class2);
    registry.register("java/lang/ClassLoader$NativeLibrary", "load", "(Ljava/lang/String;Z)V", delegate_native_lib_load);
}

gen_delegate!(delegate_find_loaded_class0, |ctx, _obj_ref, args| {
    debug!("findLoadedClass0 {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = ctx.vm.extract_string_from_value(*str_object)?;
        let class_name = class_name.replace(".", "/");
        if ctx.vm.class_manager.find_class_by_name(class_name.as_str()).is_some() {
            non_failing_some(Value::Reference(wrap_init!(ctx, ctx.new_class_object_by_name(class_name.as_str())?).id))
        } else {
            non_failing_some(ctx.vm.null())
        }
    } else {
        invalidation!("expected a string reference")
    }
});

gen_delegate!(delegate_find_bootstrap_class, |ctx, _obj_ref, args| {
    debug!("findBootstrapClass {:?}", args);
    if let Some(str_object) = args.get(0) {
        let class_name = ctx.vm.extract_string_from_value(*str_object)?;
        let class_name = class_name.replace(".", "/");

        ctx.thread.call_stack.class_loaders.borrow_mut().push(None);
        let res = match ctx.get_or_resolve_class(class_name.as_str()) {
            Ok(clazz) => non_failing_some(Value::Reference(wrap_init!(ctx, ctx.new_class_object_by_class(clazz)?).id)),
            Err(_) => non_failing_some(ctx.vm.null())
        };
        ctx.thread.call_stack.class_loaders.borrow_mut().pop();
        res
    } else {
        invalidation!("expected a string reference")
    }
});

gen_delegate!(delegate_find_builtin_lib, |ctx, _obj_ref, args| {
    debug!("findBuiltinLib {:?}", args);
    //FIXME here we have to check if the library with the given name is builtin -> exports the function JNI_OnLoad_<libname>
    non_failing_some(ctx.vm.null())
});

gen_delegate!(delegate_native_lib_load, |ctx, obj_ref, _args| {
    debug!("nativeLib::load {:?}", obj_ref);
    if let Some(obj_ref) = obj_ref {
        //handle
        obj_ref.set_field(CLASSLOADER_NATIVELIBRARY_handle_INDEX, Value::Long(1));
        let name_val = obj_ref.get_field(CLASSLOADER_NATIVELIBRARY_name_INDEX);//args.get(0).unwrap();
        let name = ctx.vm.extract_string_from_value(name_val)?;
        debug!(target: "native", "native_lib: name: {name}");
        debug!(target: "native", "native_lib: javavm: {:p}", &ctx.thread.java_vm);

        unsafe {
            use libffi::middle::{Arg, Cif, Type};
            use std::{ffi::c_void, ptr};
            //let lib = Library::new("/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/libjava.so").unwrap();
            let lib = Library::new(name).unwrap();
            let sym: Symbol<*const ()> = lib.get(b"JNI_OnLoad").unwrap();

            let func_ptr = *sym as * const c_void;
            ctx.vm.native_method_registry.add_loaded_library(lib);

            let vm_ptr = ptr::from_ref(ctx.thread.java_vm.as_ref().get_ref()) as *const c_void;
            debug!(target: "native", "native_lib: javavmp: {:p}", vm_ptr);
            let reserved = std::ptr::null() as *const c_void;
            let cif = Cif::new(vec![Type::pointer(), Type::pointer()], Type::i32()); //JNI_OnLoad
            let res: i32 = cif.call(libffi::low::CodePtr::from_ptr(func_ptr), &[Arg::new(&vm_ptr), Arg::new(&reserved)]);
            debug!(target: "native", "native_lib: res: {:x}", res);
        }
        obj_ref.set_field(CLASSLOADER_NATIVELIBRARY_loaded_INDEX, Value::from(true));

        non_failing_none()
    } else {
        invalidation!("this is null")
    }
});

gen_delegate!(delegate_define_class0, |ctx, obj_ref, args| {
    unimplemented!()
});

gen_delegate!(delegate_define_class1, |ctx, obj_ref, args| {
    if let (Some(loader_ref), Some(name_val), Some(Value::Reference(b_arr_ref_id)), Some(Value::Integer(off)), Some(Value::Integer(len)), Some(source_val)) = (obj_ref, args.get(0), args.get(1), args.get(2), args.get(3), args.get(5)) {
        let name = ctx.vm.extract_string_from_value(*name_val)?;
        let name = name.replace(".", "/");
        // seems to be nullable
        // let source = ctx.vm.extract_string_from_value(*source_val)?;
        let b_arr_ref = ctx.vm.resolve_object_by_id(*b_arr_ref_id)?;
        let bytes = if let ReferenceType::Array(data) = &b_arr_ref.reference_type{
            data.read().as_byte_vec().unwrap()
        } else {
            Vec::new()
        };
        let bytes = bytes.into_iter().skip(*off as usize).take((*len) as usize).collect::<Vec<_>>();
        ctx.thread.call_stack.class_loaders.borrow_mut().push(Some(loader_ref.id));
        let class_object = wrap_init!(ctx, ctx.define_class(name.as_str(), bytes.clone())?);
        ctx.thread.call_stack.class_loaders.borrow_mut().pop();
        non_failing_some(Value::Reference(class_object.id))
    } else {
        invalidation!("Expected Classloader(this), String, byte arr, int, int, _, String")
    }
});

gen_delegate!(delegate_define_class2, |ctx, obj_ref, args| {
    unimplemented!()
});