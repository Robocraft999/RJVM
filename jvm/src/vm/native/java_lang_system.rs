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
use crate::vm::java_thread::JavaThread;

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

gen_delegate!(delegate_nano_time, |_ctx, _obj_ref, _args| {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    non_failing_some(Value::Long(nanos))
});

gen_delegate!(delegate_time_millis, |_ctx, _obj_ref, _args| {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    non_failing_some(Value::Long(millis))
});

gen_delegate!(delegate_identity_hash_code, |_ctx, _obj_ref, args| {
    if let Some(Value::Reference(object_id)) = args.get(0){
        let mut hasher = DefaultHasher::new();
        object_id.hash(&mut hasher);
        let addr = hasher.finish() as i32;
        trace!(target: "native", "HASH: {addr} {object_id:?}");
        non_failing_some(Value::Integer(addr))
    } else {
        invalidation!("Expected Object but found '{:?}'", args.get(0))
    }
});

gen_delegate!(delegate_set_out0, |ctx, _obj_ref, args| {
    let clazz = ctx.vm.get_or_resolve_class(JAVA_LANG_SYSTEM)?;
    if let Some(static_obj_refect) = ctx.vm.get_static_class_object(clazz.id){
        if let Some(Value::Reference(object)) = args.get(0){
            static_obj_refect.set_field(SYSTEM_out_INDEX, Value::Reference(*object));
            non_failing_none()
        } else {
            invalidation!("Expected Object but found '{:?}'", args.get(0))
        }
    } else {
        invalidation!("Couldn't find static Object of class {}", clazz.name)
    }
});

gen_delegate!(delegate_set_err0, |ctx, _obj_ref, args| {
    let clazz = ctx.vm.get_or_resolve_class(JAVA_LANG_SYSTEM)?;
    if let Some(static_obj_refect) = ctx.vm.get_static_class_object(clazz.id){
        if let Some(Value::Reference(object_id)) = args.get(0){
            static_obj_refect.set_field(SYSTEM_err_INDEX, Value::Reference(*object_id));
            non_failing_none()
        } else {
            invalidation!("Expected Object but found '{:?}'", args.get(0))
        }
    } else {
        invalidation!("Couldn't find static Object of class {}", clazz.name)
    }
});

// TODO real arraycopy
gen_delegate!(delegate_arraycopy, |ctx, _obj_ref, args| {
    if let (
        Some(Value::Reference(src_id)),
        Some(Value::Integer(src_pos)),
        Some(Value::Reference(dst_id)),
        Some(Value::Integer(dst_pos)),
        Some(Value::Integer(length))
    ) = (args.get(0), args.get(1), args.get(2), args.get(3), args.get(4)){
        let src_ref = ctx.vm.resolve_object_by_id(*src_id)?;
        let dst_ref = ctx.vm.resolve_object_by_id(*dst_id)?;
        let src_pos = *src_pos as usize;
        let dst_pos = *dst_pos as usize;
        if let (ReferenceType::Array(_, _, src), ReferenceType::Array(_, _, dst)) = (&src_ref.reference_type, &dst_ref.reference_type){
            let length = *length as usize;
            // if src and dst are the same, we must preserve the original content before we start copying.
            let src_content = src.read()?[src_pos..src_pos + length].to_vec();
            dst.write()?[dst_pos..dst_pos+length].clone_from_slice(&src_content);
            ctx.thread.debug_helper.tracker.push_object_event(dst_ref.id, format!("Arraycopy from {:?} [{}:{}]->[{}:{}] :\n    {:?}", src_ref.id, src_pos, src_pos+length, dst_pos, dst_pos+length, dst_ref));
            return non_failing_none()
        }
    }
    invalidation!("Expected two arrays with indices")
});

gen_delegate!(delegate_init_properties, |ctx, _obj_ref, args| {
    let Some(Value::Reference(properties_obj_id)) = args.get(0) else { return invalidation!("this properties is not a reference") };
    let properties_ref = ctx.vm.resolve_object_by_id(*properties_obj_id)?;
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
    let properties_set_method = ctx.vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    let current_frame_index = ctx.thread.call_stack.frames.borrow().len() as isize - 1;
    for (key, value) in props.into_iter(){
        //FIXME could be bad to unwrap
        let arg1 = ctx.vm.try_new_string_object(key)?;
        let arg2 = ctx.vm.try_new_string_object(value.as_str())?;
        ctx.thread.call_stack.create_and_push_call_frame(properties_set_method.clone(), Some(properties_ref), vec![Value::Reference(arg1.id), Value::Reference(arg2.id)], false)
    }
    let _res = JavaThread::invoke_frames_until(ctx, current_frame_index)?;
    //Ok(VMResultType::NeedsClassInit(frames, false))
    non_failing_some(ctx.vm.null())
});

gen_delegate!(delegate_system_map_library_name, |ctx, _obj_ref, args| {
    if let Some(string) = args.get(0) {
        let name = ctx.vm.extract_string_from_value(*string)?;
        let new_name = match env::consts::OS{
            "windows" => name + ".dll",
            "linux" => format!("lib{name}.so"),
            _ => name
        };
        let string_ref = wrap_init!(ctx, ctx.vm.new_string_object(new_name.as_str())?);
        non_failing_some(Value::Reference(string_ref.id))
    } else {
        invalidation!("Expected Reference but found '{:?}'", args.get(0))
    }
});