use crate::vm::constants::classes::JAVA_LANG_SYSTEM;
use crate::vm::constants::{SYSTEM_err_INDEX, SYSTEM_out_INDEX};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VmError, VM};
use log::trace;
use std::env;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate|registry.register(JAVA_LANG_SYSTEM, method_name, sig, delegate);
    register("nanoTime", "()J", delegate_nano_time);
    register("currentTimeMillis", "()J", delegate_time_millis);
    register("identityHashCode", "(Ljava/lang/Object;)I", delegate_identity_hash_code);
    register("setOut0", "(Ljava/io/PrintStream;)V", delegate_set_out0);
    register("setErr0", "(Ljava/io/PrintStream;)V", delegate_set_err0);
    register("arraycopy", "(Ljava/lang/Object;ILjava/lang/Object;II)V", delegate_arraycopy);
    register("initProperties", "(Ljava/util/Properties;)Ljava/util/Properties;", delegate_init_properties);
    register("mapLibraryName", "(Ljava/lang/String;)Ljava/lang/String;", delegate_system_map_library_name);
}

gen_delegate!(delegate_nano_time, |_vm, _java_vm, _obj_ref, _args| {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    non_failing_some(Value::Long(nanos))
});

gen_delegate!(delegate_time_millis, |_vm, _java_vm, _obj_ref, _args| {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    non_failing_some(Value::Long(millis))
});

gen_delegate!(delegate_identity_hash_code, |_vm, _java_vm, _obj_ref, args| {
    if let Some(Value::Reference(object)) = args.get(0){
        let mut hasher = DefaultHasher::new();
        object.id.hash(&mut hasher);
        let addr = hasher.finish() as i32;
        trace!(target: "native", "HASH: {addr} {object:?}");
        non_failing_some(Value::Integer(addr))
    } else {
        invalidation!("Expected Object but found '{:?}'", args.get(0))
    }
});

gen_delegate!(delegate_set_out0, |vm, _java_vm, _obj_ref, args| {
    let clazz = vm.get_or_resolve_class(JAVA_LANG_SYSTEM)?;
    if let Some(static_obj_refect) = vm.get_static_class_object(clazz.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_obj_refect.set_field(SYSTEM_out_INDEX, Value::Reference(object));
            non_failing_none()
        } else {
            invalidation!("Expected Object but found '{:?}'", args.get(0))
        }
    } else {
        invalidation!("Couldn't find static Object of class {}", clazz.name)
    }
});

gen_delegate!(delegate_set_err0, |vm, _java_vm, _obj_ref, args| {
    let clazz = vm.get_or_resolve_class(JAVA_LANG_SYSTEM)?;
    if let Some(static_obj_refect) = vm.get_static_class_object(clazz.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_obj_refect.set_field(SYSTEM_err_INDEX, Value::Reference(object));
            non_failing_none()
        } else {
            invalidation!("Expected Object but found '{:?}'", args.get(0))
        }
    } else {
        invalidation!("Couldn't find static Object of class {}", clazz.name)
    }
});

// TODO real arraycopy
gen_delegate!(delegate_arraycopy, |vm, _java_vm, _obj_ref, args| {
    if let (
        Some(Value::Reference(src_ref)),
        Some(Value::Integer(src_pos)),
        Some(Value::Reference(dst_ref)),
        Some(Value::Integer(dst_pos)),
        Some(Value::Integer(length))
    ) = (args.get(0), args.get(1), args.get(2), args.get(3), args.get(4)){
        let src_pos = *src_pos as usize;
        let dst_pos = *dst_pos as usize;
        if let (ReferenceType::Array(_, _, src), ReferenceType::Array(_, _, dst)) = (&src_ref.reference_type, &dst_ref.reference_type){
            let length = *length as usize;
            // if src and dst are the same, we must preserve the original content before we start copying.
            let src_content = src.borrow()[src_pos..src_pos + length].to_vec();
            dst.borrow_mut()[dst_pos..dst_pos+length].clone_from_slice(&src_content);
            vm.debug_helper.tracker.push_object_event(dst_ref.id, format!("Arraycopy from {} [{}:{}]->[{}:{}] :\n    {:?}", src_ref.id, src_pos, src_pos+length, dst_pos, dst_pos+length, dst_ref));
            return non_failing_none()
        }
    }
    invalidation!("Expected two arrays with indices")
});

gen_delegate!(delegate_init_properties, |vm, java_vm, _obj_ref, args| {
    let properties_obj_refect = args.get(0).unwrap().expect_reference()?;
    let mut props = vec![
        ("file.encoding", "UTF-8".to_string()),
        ("line.separator", "\n".to_string()),
        ("file.separator", "/".to_string()),
        ("path.separator", ":".to_string()),
        ("java.lang.Integer.IntegerCache.high", "127".to_string()),
        //("sun.boot.library.path", "/home/admin/.jdks/temurin-22.0.1/lib".to_string()),
        ("java.home", format!("{}/jre/", env!("JAVA_HOME"))),
        ("sun.boot.library.path", format!("{}/jre/lib/amd64/", env!("JAVA_HOME"))),
        ("sun.boot.class.path", "resources/rt.jar:resources/resources.jar".to_string()),
        ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
        ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
        ("os.name", "Linux".to_string()),
        ("os.arch", "x86_64".to_string()),
        ("java.awt.graphicsenv", "sun.awt.X11GraphicsEnvironment".to_owned()),
    ];
    if env::consts::OS == "windows"{
        props = vec![
            ("file.encoding", "UTF-8".to_string()),
            ("line.separator", "\r\n".to_string()),
            ("file.separator", "\\\\".to_string()),
            ("path.separator", ";".to_string()),
            ("java.lang.Integer.IntegerCache.high", "127".to_string()),
            ("sun.boot.library.path", "C:\\Users\\Admin\\.jdks\\azul-22.0.1\\bin".to_string()),
            ("user.dir", env::current_dir().unwrap().to_string_lossy().to_string()),
            ("user.home", env::home_dir().unwrap().to_string_lossy().to_string()),
            ("os.name", "Windows".to_string()),
        ];
    }
    let properties_set_method = vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let current_frame_index = vm.call_stack.frames.borrow().len() as isize - 1;
    for (key, value) in props.into_iter(){
        //FIXME could be bad to unwrap
        let arg1 = vm.try_new_string_object(key)?;
        let arg2 = vm.try_new_string_object(value.as_str())?;
        vm.call_stack.create_and_push_call_frame(properties_set_method.clone(), Some(properties_obj_refect), vec![Value::Reference(arg1), Value::Reference(arg2)], false)
    }
    let _res = vm.invoke_frames_until(java_vm, current_frame_index)?;
    //Ok(VMResultType::NeedsClassInit(frames, false))
    non_failing_some(vm.null())
});

gen_delegate!(delegate_system_map_library_name, |vm, java_vm, _obj_ref, args| {
    if let Some(string) = args.get(0) {
        let name = VM::extract_string_from_object(string)?;
        let new_name = match env::consts::OS{
            "windows" => name + ".dll",
            "linux" => format!("lib{name}.so"),
            _ => name
        };
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(new_name.as_str())?)))
    } else {
        invalidation!("Expected Reference but found '{:?}'", args.get(0))
    }
});