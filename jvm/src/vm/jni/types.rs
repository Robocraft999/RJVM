#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_variables)]

use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::native_init_wrap;
use crate::vm::application::thread;
use crate::vm::class::{ClassAndMethod, ClassId};
use crate::vm::constants::THROWABLE_detailsMessage_INDEX;
use crate::vm::java_thread::JavaThread;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{RefId, ReferenceType, Value};
use crate::vm::{jni, jni::{env_function_table::JNINativeInterface, vm_function_table::JNIInvokeInterface}, Context, VmError, VM};
use log::{debug, error, warn};
use parking_lot::RwLock;
use std::ffi::{c_char, c_double, c_float, c_int, c_long, c_schar, c_short, c_uchar, c_ushort, c_void, CStr, CString, OsStr, VaList};
use std::fmt::Debug;
use std::os::unix::ffi::OsStrExt;
use std::{ptr, slice};

//Platform dependent
pub type jint = c_int;
pub type jlong = c_long;
pub type jbyte = c_schar;

//Primitives
pub type jboolean = c_uchar;
pub type jchar = c_ushort;
pub type jshort = c_short;
pub type jfloat = c_float;
pub type jdouble = c_double;
pub type jsize = jint;

//Object related
#[repr(C)]
pub struct _jobject;
pub type jobject = u32;
pub type jclass = jobject;
pub type jthrowable = jobject;
pub type jstring = jobject;
pub type jarray = jobject;
pub type jbooleanArray = jarray;
pub type jbyteArray = jarray;
pub type jcharArray = jarray;
pub type jshortArray = jarray;
pub type jintArray = jarray;
pub type jlongArray = jarray;
pub type jfloatArray = jarray;
pub type jdoubleArray = jarray;
pub type jobjectArray = jarray;

//Value
pub type jweak = jobject;

#[repr(C)]
pub union jvalue{
    pub z: jboolean,
    pub b: jbyte,
    pub c: jchar,
    pub s: jshort,
    pub i: jint,
    pub j: jlong,
    pub f: jfloat,
    pub d: jdouble,
    pub l: jobject,
}

#[repr(C)]
pub struct _jfieldID;
pub type jfieldID = usize;
#[repr(C)]
pub struct _jmethodID;
pub type jmethodID = usize;

/*
 * jboolean constants
 */
const JNI_FALSE: jboolean = 0;
const JNI_TRUE: jboolean = 1;

/*
 * possible return values for JNI functions.
 */

const JNI_OK: i32 =          0;                 /* success */
const JNI_ERR: i32 =         (-1);              /* unknown error */
const JNI_EDETACHED: i32 =   (-2);              /* thread detached from the VM */
const JNI_EVERSION: i32 =    (-3);              /* JNI version error */
const JNI_ENOMEM: i32 =      (-4);              /* not enough memory */
const JNI_EEXIST: i32 =      (-5);              /* VM already created */
const JNI_EINVAL: i32 =      (-6);              /* invalid arguments */

/*
 * used in ReleaseScalarArrayElements
 */

const JNI_COMMIT: i32 = 1;
const JNI_ABORT: i32 =  2;

/*
 * used in RegisterNatives to describe native method name, signature,
 * and function pointer.
 */
#[repr(C)]
pub struct JNINativeMethod{
    name:      *const c_char,
    signature: *const c_char,
    fnPtr:     *const c_void,
}

/*
 * JNI Native Method Interface.
 */
#[repr(C)]
pub struct JNIEnv{
    pub(crate) methods: *const JNINativeInterface,
    pub vm: *const (),
}

impl JNIEnv {
    pub fn new(vm_ptr: *const ()) -> Self {
        Self {
            methods: jni::env_function_table::METHODS,
            vm: vm_ptr
        }
    }

    pub fn vm(&self) -> &VM {
        unsafe{&*(self.vm as *const VM)}
    }
}
impl !Unpin for JNIEnv{}

unsafe impl Send for JNIEnv {}
unsafe impl Sync for JNIEnv {}

/*
 * JNI Invocation Interface.
 */

#[repr(C)]
pub struct JavaVM{
    pub methods: *const JNIInvokeInterface,
}

impl JavaVM {
    pub fn new() -> Self {
        Self {
            methods: jni::vm_function_table::METHODS,
        }
    }
}

impl !Unpin for JavaVM{}
unsafe impl Send for JavaVM {}
unsafe impl Sync for JavaVM {}

#[repr(C)]
pub struct JNINativeInterface_{}

macro_rules! native_wrap {
    ($code:block, $ret:ty) => {
        fn t() -> VMResult<$ret> {
            $code
        }
        match t(){
            Ok(v) => v,
            Err(e) => {
                let vm = unsafe{&*(*env).vm};
                vm.native_method_registry.mark_exception();
                panic!("this should be able to break out of here");
            }
        }
    };
}

unsafe fn resolve_static_class_and_method<'a>(env: *mut JNIEnv, clazz: jclass, method_id: jmethodID) -> ClassAndMethod<'a>{
    let vm = unsafe{(*env).vm()};

    let class_ref = vm.resolve_class_object_by_jclass(clazz);

    let method_info = class_ref.methods.get(method_id).unwrap();
    ClassAndMethod{class: class_ref, method: method_info}
}

unsafe fn resolve_class_and_method<'a>(env: *mut JNIEnv, obj: jobject, method_id: jmethodID) -> ClassAndMethod<'a>{
    let vm = unsafe{(*env).vm()};

    let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
    let class_ref = vm.get_or_resolve_class(obj_ref.class_name.as_str()).unwrap();

    let method_info = class_ref.methods.get(method_id).unwrap();
    ClassAndMethod{class: class_ref, method: method_info}
}

unsafe fn resolve_function_args<'a>(env: *mut JNIEnv, class_and_method: &ClassAndMethod, mut raw: VaList) -> Vec<Value>{
    let vm = unsafe{(*env).vm()};
    class_and_method.method.descriptor.args.iter().flat_map(|ft| match ft{
        FieldType::Object(..) | FieldType::Array(..) => {
            let ref_id: u32 = unsafe{raw.next_arg()};
            {
                let reference = Value::Reference(RefId(ref_id));
                println!("NATIVE: arg for {}: {:?}", class_and_method.format(), reference.print(vm));
                vec![reference]
            }
        }
        FieldType::Primitive(pt) => match pt{
            PrimitiveType::Boolean | PrimitiveType::Byte | PrimitiveType::Char | PrimitiveType::Integer | PrimitiveType::Short => {
                let i: i32 = unsafe{raw.next_arg()};
                vec![Value::Integer(i)]
            }
            PrimitiveType::Float => {
                let f: f64 = unsafe{raw.next_arg()};
                vec![Value::Float(f as f32)]
            }
            PrimitiveType::Double => {
                let d: f64 = unsafe{raw.next_arg()};
                vec![Value::Double(d), Value::Dummy]
            }
            PrimitiveType::Long => {
                let l: i64 = unsafe{raw.next_arg()};
                vec![Value::Long(l), Value::Dummy]
            }
        }
    }).collect()
}

impl JNINativeInterface_ {
    pub fn GetVersion(env: *mut JNIEnv) -> jint{
        unimplemented!()
    }

    pub fn DefineClass(env: *mut JNIEnv, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize) -> jclass {
        unimplemented!()
    }
    pub unsafe extern "system-unwind" fn FindClass(env: *mut JNIEnv, name: *const c_char) -> jclass {
        let name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let name = name.replace(".", "/");
        debug!(target: "native", "NATIVE: FindClass: '{}'", name);
        if name == ""{
            0 as jclass
        } else {
            let vm = unsafe{(*env).vm()};
            let class = if let Some(class) = vm.find_class_by_name(name.as_str()){
                class
            } else {
                vm.get_or_resolve_class(name.as_str()).unwrap()
            };
            let class_obj = native_init_wrap!(env, vm.new_class_object_by_class(class));
            class_obj.id.nid()
        }
    }

    pub fn FromReflectedMethod(env: *mut JNIEnv, method: jobject) -> jmethodID {
        unimplemented!()
    }
    pub fn FromReflectedField(env: *mut JNIEnv, field: jobject) -> jfieldID {
        unimplemented!()
    }

    pub fn ToReflectedMethod(env: *mut JNIEnv, cls: jclass, method: jmethodID, isStatic: jboolean) -> jobject {
        unimplemented!()
    }

    pub fn GetSuperclass(env: *mut JNIEnv, sub: jclass) -> jclass{
        unimplemented!()
    }
    pub fn IsAssignableFrom(env: *mut JNIEnv, sub: jclass, sup: jclass) -> jboolean {
        unimplemented!()
    }

    pub fn ToReflectedField(env: *mut JNIEnv, cls: jclass, field: jfieldID, isStatic: jboolean) -> jobject {
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn Throw(env: *mut JNIEnv, obj: jthrowable) -> jint{
        let vm: &VM = unsafe{(*env).vm()};
        debug!(target: "native", "NATIVE: Throw");
        let object = vm.resolve_object_by_jobject(obj).unwrap();
        let prev = thread().caught_exception.replace(Some(("NativeException".to_string(), "Unknown".to_string(), Value::Reference(object.id))));
        assert!(prev.is_none());
        0 as jint
    }
    pub unsafe extern "system-unwind" fn ThrowNew(env: *mut JNIEnv, clazz: jclass, msg: *const c_char) -> jint{
        let vm: &VM = unsafe{(*env).vm()};
        debug!(target: "native", "NATIVE: ThrowNew");

        let msg = if !msg.is_null() {
            unsafe { CStr::from_ptr(msg).to_owned().into_string().map_err(|e| VmError::Native(e.to_string())).unwrap() }
        } else {
            String::new()
        };

        let ctx = Context { thread: thread(), vm };

        let clazz = vm.resolve_class_object_by_jclass(clazz);
        native_init_wrap!(env, ctx.ensure_initialized(clazz));
        let exception_object = ctx.vm.new_object_from_class(clazz);

        let details = ctx.vm.try_new_string_object(msg.as_str()).unwrap();
        //detailsMessage
        exception_object.set_field(THROWABLE_detailsMessage_INDEX, Value::Reference(details.id));

        let prev = ctx.thread.caught_exception.replace(
            Some((
                msg,
                "<Native>::ThrowNew".to_owned(),
                Value::Reference(exception_object.id)
            )));
        assert!(prev.is_none());

        0 as jint
    }
    pub fn ExceptionOccurred(env: *mut JNIEnv) -> jthrowable{
        let vm: &VM = unsafe{(*env).vm()};
        if let Some((_, _, Value::Reference(throwable))) = &thread().caught_exception.borrow().as_ref(){
            throwable.nid() as jthrowable
        } else {
            0 as jthrowable
        }
    }
    pub fn ExceptionDescribe(env: *mut JNIEnv){
        unimplemented!()
    }
    pub fn ExceptionClear(env: *mut JNIEnv){
        let vm: &VM = unsafe{(*env).vm()};
        let old = thread().caught_exception.replace(None);
        if let Some(e) = old{
            warn!(target: "native", "an Exception was cleared: {:?}", e)
        }
    }
    pub fn FatalError(env: *mut JNIEnv, msg: *const c_char){
        error!("Fatal Error: '{:?}'", unsafe {CStr::from_ptr(msg)});
        panic!()
    }

    pub fn PushLocalFrame(env: *mut JNIEnv, capacity: jint) -> jint{
        unimplemented!()
    }
    pub fn PopLocalFrame(env: *mut JNIEnv, result: jobject) -> jint{
        unimplemented!()
    }

    pub fn NewGlobalRef(env: *mut JNIEnv, lobj: jobject) -> jobject{
        let vm = unsafe{(*env).vm()};
        debug!("NATIVE: NewGlobalRef: {:?}", vm.resolve_object_by_jobject(lobj));
        //FIXME currently the object exits forever so this is fine
        lobj
    }
    pub fn DeleteGlobalRef(env: *mut JNIEnv, gref: jobject){
        unimplemented!()
    }
    pub fn DeleteLocalRef(env: *mut JNIEnv, obj: jobject){
        debug!("NATIVE: DeleteLocalRef: {}", obj);
    }
    pub fn IsSameObject(env: *mut JNIEnv, obj1: jobject, obj2: jobject) -> jboolean{
        unimplemented!()
    }
    pub fn NewLocalRef(env: *mut JNIEnv, r#ref: jobject) -> jobject{
        unimplemented!()
    }
    pub fn EnsureLocalCapacity(env: *mut JNIEnv, capacity: jint) -> jint{
        debug!("NATIVE: EnsureLocalCapacity: amount:{}", capacity);
        //TODO
        JNI_OK
    }

    pub fn AllocObject(env: *mut JNIEnv, clazz: jclass) -> jobject{
        unimplemented!()
    }
    pub unsafe extern "C-unwind" fn NewObject(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, mut args: ...) -> jobject{
        unsafe{Self::NewObjectV(env, clazz, methodID, args)}
    }
    pub unsafe extern "system-unwind" fn NewObjectV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: VaList) -> jobject{
        let vm = unsafe{(*env).vm()};
        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context { vm, thread: thread() };

        let _ = native_init_wrap!(env, ctx.ensure_initialized(class_and_method.class));
        let obj_ref = vm.new_object_from_class(class_and_method.class);
        debug!("NewObjectV: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, Some(obj_ref), args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                vm.native_method_registry.mark_exception();
                return 0;
            }
        };
        if let VMResultType::Successful(None) = res{
            obj_ref.id.nid() as jobject
        } else {
            unimplemented!("NewObjectV: expected no return value but got: {:?}", res)
        }
    }
    pub unsafe extern "system-unwind" fn NewObjectA(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetObjectClass(env: *mut JNIEnv, obj: jobject) -> jclass{
        let vm: &VM = unsafe{(*env).vm()};
        if let Some(object) = vm.resolve_object_by_jobject(obj){
            let class_obj = native_init_wrap!(env, vm.new_class_object(object.class_name.as_str(), object.class_id));
            class_obj.id.nid() as jclass
        } else {
            0 as jclass
        }

    }
    pub unsafe extern "system-unwind" fn IsInstanceOf(env: *mut JNIEnv, obj: jobject, clazz: jclass) -> jboolean{
        let vm = unsafe{(*env).vm()};
        if obj == 0{
            // FIXME this is opposite of what is stated here: https://docs.oracle.com/en/java/javase/23/docs/specs/jni/functions.html#isinstanceof
            // but allowing true breaks stuff
            return JNI_FALSE;
        }
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let obj_class = vm.find_class_by_id(obj_ref.class_id).unwrap();
        let class_ref = vm.resolve_class_object_by_jclass(clazz);
        let instance_of = vm.is_instance_of(obj_class, class_ref);
        if instance_of {JNI_TRUE} else {JNI_FALSE}
    }

    pub unsafe extern "system-unwind" fn GetMethodID(env: *mut JNIEnv, clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        let method_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{(*env).vm()};

        let class_ref = vm.resolve_class_object_by_jclass(clazz);
        println!("NATIVE: GetMethodID: {}::{}{}", class_ref.name, method_name, signature);
        //FIXME zero index results in NULL
        class_ref.find_method_index(method_name, signature).ok_or(VmError::Native(format!("GetMethodID: {}::{}{} not found", class_ref.name, method_name, signature))).unwrap()
    }

    pub unsafe extern "C-unwind" fn CallObjectMethod(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, mut params: ...) -> jobject{
        unsafe{Self::CallObjectMethodV(env, obj, methodID, params)}
    }

    pub unsafe extern "system-unwind" fn CallObjectMethodV(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, args: VaList) -> jobject{
        let vm = unsafe{(*env).vm()};
        let class_and_method = unsafe{ resolve_class_and_method(env, obj, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context { vm, thread: thread()};

        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        debug!("CallObjectMethodV: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, Some(obj_ref), args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                ctx.thread.debug_helper.print();
                vm.native_method_registry.mark_exception();
                return 0;
            }
        };
        if let VMResultType::Successful(Some(Value::Reference(reference))) = res{
            reference.nid() as jobject
        } else {
            unimplemented!("CallObjectMethodV: expected object return value but got: {:?}", res)
        }
    }

    pub fn CallObjectMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }
    pub fn CallBooleanMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jboolean{
        unimplemented!()
    }
    pub fn CallByteMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jbyte{
        unimplemented!()
    }
    pub fn CallCharMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jchar{
        unimplemented!()
    }
    pub fn CallShortMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jshort{
        unimplemented!()
    }
    pub fn CallIntMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jint{
        unimplemented!()
    }
    pub fn CallLongMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jlong{
        unimplemented!()
    }
    pub fn CallFloatMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jfloat{
        unimplemented!()
    }
    pub fn CallDoubleMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue) -> jdouble{
        unimplemented!()
    }

    pub unsafe extern "C-unwind" fn CallVoidMethod(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, mut params: ...){
        unsafe{Self::CallVoidMethodV(env, obj, methodID, params)}
    }

    pub unsafe extern "system-unwind" fn CallVoidMethodV(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, args: VaList){
        let vm = unsafe{(*env).vm()};
        let class_and_method = unsafe{ resolve_class_and_method(env, obj, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context { vm, thread: thread()};

        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        debug!("CallVoidMethodV: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, Some(obj_ref), args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                ctx.thread.debug_helper.print();
                vm.native_method_registry.mark_exception();
                return;
            }
        };
        if let VMResultType::Successful(None) = res{
            //works
        } else {
            unimplemented!("CallVoidMethodV: expected no return value but got: {:?}", res)
        }
    }
    pub fn CallVoidMethodA(obj: jobject, methodID: jmethodID, args: *const jvalue){
        unimplemented!()
    }

    pub fn CallNonvirtualObjectMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }
    pub fn CallNonvirtualBooleanMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jboolean{
        unimplemented!()
    }
    pub fn CallNonvirtualByteMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jbyte{
        unimplemented!()
    }
    pub fn CallNonvirtualCharMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jchar{
        unimplemented!()
    }
    pub fn CallNonvirtualShortMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jshort{
        unimplemented!()
    }
    pub fn CallNonvirtualIntMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jint{
        unimplemented!()
    }
    pub fn CallNonvirtualLongMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jlong{
        unimplemented!()
    }
    pub fn CallNonvirtualFloatMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jfloat{
        unimplemented!()
    }
    pub fn CallNonvirtualDoubleMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jdouble{
        unimplemented!()
    }
    pub fn CallNonvirtualVoidMethodA(obj: jobject, clazz: jclass, methodID: jmethodID, args: *const jvalue){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetFieldID(env: *mut JNIEnv, clazz: jclass, name: *const c_char, sig: *const c_char) -> jfieldID{
        let field_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{(*env).vm()};

        let class_ref = vm.resolve_class_object_by_jclass(clazz);
        println!("NATIVE: GetFieldID: {}::{}{}", class_ref.name, field_name, signature);
        // FIXME same as GetMethodID, there is a field at index 0 which is recognized as NULL
        if let Some((index, _)) = class_ref.find_field(field_name){
            index as jfieldID
        } else {
            let message = format!("GetMethodID: {}::{} not found", class_ref.name, field_name);
            // NoSuchFieldError
            let prev = thread().caught_exception.replace(Some((message, "JNI_GetFieldID".to_string(), vm.null())));
            0 as jfieldID
        }
    }

    pub unsafe extern "system-unwind" fn GetObjectField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jobject{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Reference(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val.nid()
    }
    pub unsafe extern "system-unwind" fn GetBooleanField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jboolean{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Integer(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jboolean
    }
    pub unsafe extern "system-unwind" fn GetByteField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jbyte{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Integer(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jbyte
    }
    pub unsafe extern "system-unwind" fn GetCharField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jchar{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Integer(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jchar
    }
    pub unsafe extern "system-unwind" fn GetShortField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jshort{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Integer(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jshort
    }
    pub unsafe extern "system-unwind" fn GetIntField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jint{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Integer(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jint
    }
    pub unsafe extern "system-unwind" fn GetLongField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jlong{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Long(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jlong
    }
    pub unsafe extern "system-unwind" fn GetFloatField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jfloat{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Float(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jfloat
    }
    pub unsafe extern "system-unwind" fn GetDoubleField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID) -> jdouble{
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        let Value::Double(val) = obj_ref.get_field(fieldID as usize) else { unreachable!("") };
        val as jdouble
    }

    unsafe fn SetField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: Value) {
        let vm = unsafe{(*env).vm()};
        let obj_ref = vm.resolve_object_by_jobject(obj).unwrap();
        obj_ref.set_field(fieldID as usize, val);
    }
    pub unsafe extern "system-unwind" fn SetObjectField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jobject) {
        let vm = unsafe{(*env).vm()};
        let val = if val != 0 {
            Value::Reference(RefId(obj))
        } else {
            vm.null()
        };
        unsafe { Self::SetField(env, obj, fieldID, val) }
    }
    pub unsafe extern "system-unwind" fn SetBooleanField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jboolean) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Integer(val as i32)) }
    }
    pub unsafe extern "system-unwind" fn SetByteField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jbyte) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Integer(val as i32)) }
    }
    pub unsafe extern "system-unwind" fn SetCharField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jchar) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Integer(val as i32)) }
    }
    pub unsafe extern "system-unwind" fn SetShortField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jshort) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Integer(val as i32)) }
    }
    pub unsafe extern "system-unwind" fn SetIntField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jint) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Integer(val as i32)) }
    }
    pub unsafe extern "system-unwind" fn SetLongField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jlong) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Long(val as i64)) }
    }
    pub unsafe extern "system-unwind" fn SetFloatField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jfloat) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Float(val as f32)) }
    }
    pub unsafe extern "system-unwind" fn SetDoubleField(env: *mut JNIEnv, obj: jobject, fieldID: jfieldID, val: jdouble) {
        unsafe { Self::SetField(env, obj, fieldID, Value::Double(val as f64)) }
    }

    pub unsafe extern "system-unwind" fn GetStaticMethodID(env: *mut JNIEnv, clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        let method_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{(*env).vm()};
        let ctx = Context { vm, thread: thread() };

        let class_ref = vm.resolve_class_object_by_jclass(clazz);
        native_init_wrap!(env, ctx.ensure_initialized(class_ref));
        println!("NATIVE: GetStaticMethodID: {}::{}{}", class_ref.name, method_name, signature);
        //FIXME zero index results in NULL
        class_ref.find_method_index(method_name, signature).ok_or(VmError::Native(format!("GetStaticMethodID: {}::{}{} not found", class_ref.name, method_name, signature))).unwrap()
    }

    pub unsafe extern "C-unwind" fn CallStaticObjectMethod(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, params: ...) -> jobject {
        unsafe { Self::CallStaticObjectMethodV(env, clazz, methodID, params) }
    }

    pub unsafe extern "system-unwind" fn CallStaticObjectMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, mut args: VaList) -> jobject {
        let vm = unsafe{(*env).vm()};

        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context { vm, thread: thread() };

        debug!("CallStaticObjectMethodV: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, None, args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                ctx.thread.debug_helper.print();
                vm.native_method_registry.mark_exception();
                return 0;
            }
        };
        if let VMResultType::Successful(Some(result)) = res{
            if let Value::Reference(r) = result{
                r.nid() as jobject
            } else {
                unreachable!("CallStaticObjectMethodV: expected an object or null return value")
            }
        } else {
            unimplemented!("CallStaticObjectMethodV: expected a return value")
        }
    }

    pub fn CallStaticObjectMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }

    pub unsafe extern "C-unwind" fn CallStaticBooleanMethod(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, params: ...) -> jboolean {
        unsafe { Self::CallStaticBooleanMethodV(env, clazz, methodID, params) }
    }

    pub unsafe extern "system-unwind" fn CallStaticBooleanMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: VaList) -> jboolean {
        let vm = unsafe{(*env).vm()};
        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context {vm, thread: thread() };

        debug!("CallStaticBooleanMethod: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, None, args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                ctx.thread.debug_helper.print();
                vm.native_method_registry.mark_exception();
                return 0;
            }
        };
        if let VMResultType::Successful(Some(result)) = res{
            if let Value::Integer(val) = result{
                assert!(val == 0 || val == 1);
                val as jboolean
            } else {
                unreachable!("CallStaticBooleanMethod: expected a boolean return value")
            }
        } else {
            unimplemented!("CallStaticBooleanMethod: expected a return value")
        }
    }

    pub unsafe extern "system-unwind" fn CallStaticBooleanMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jboolean{
        unimplemented!()
    }
    pub fn CallStaticByteMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jbyte{
        unimplemented!()
    }
    pub fn CallStaticCharMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jchar{
        unimplemented!()
    }
    pub fn CallStaticShortMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jshort{
        unimplemented!()
    }
    pub fn CallStaticIntMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jint{
        unimplemented!()
    }
    pub fn CallStaticLongMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jlong{
        unimplemented!()
    }
    pub fn CallStaticFloatMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jfloat{
        unimplemented!()
    }
    pub fn CallStaticDoubleMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jdouble{
        unimplemented!()
    }
    pub unsafe extern "C-unwind" fn CallStaticVoidMethod(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, mut params: ...){
        unsafe{Self::CallStaticVoidMethodV(env, obj, methodID, params)}
    }
    pub unsafe extern "system-unwind" fn CallStaticVoidMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: VaList){
        let vm = unsafe{(*env).vm()};
        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let ctx = Context { vm, thread: thread() };

        debug!("CallStaticVoidMethod: {} ({:?})", class_and_method.format(), args);
        let res = match JavaThread::invoke_subroutine(ctx, class_and_method, None, args) {
            Ok(result) => result,
            Err(e) => {
                error!(target: "native", "Java error: {:?}", e);
                ctx.thread.debug_helper.print();
                vm.native_method_registry.mark_exception();
                return;
            }
        };
        if let VMResultType::Successful(None) = res{
            //works
        } else {
            unimplemented!("CallStaticVoidMethod: expected no return value but got: {:?}", res)
        }
    }
    pub fn CallStaticVoidMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue){
        unimplemented!()
    }

    unsafe fn GetStaticField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> Value{
        let vm = unsafe{(*env).vm()};

        let clazz = vm.resolve_class_object_by_jclass(clazz);

        let guard = vm.static_class_objects.read();
        let class_ref = guard.get(&clazz.id).unwrap();
        class_ref.get_field(fieldID as usize)
    }

    pub unsafe extern "system-unwind" fn GetStaticFieldID(env: *mut JNIEnv, clazz: jclass, name: *mut c_char, sig: *const c_char) -> jfieldID{
        let field_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{(*env).vm()};

        let clazz = vm.resolve_class_object_by_jclass(clazz);
        println!("NATIVE: GetStaticFieldID: {}::{}{}", clazz.name, field_name, signature);
        // FIXME same as GetMethodID, there is a field at index 0 which is recognized as NULL
        if let Some((index, _, _)) = clazz.find_field_static(field_name){
            index as jfieldID
        } else {
            let message = format!("GetStaticFieldID: {}::{} not found", clazz.name, field_name);
            // NoSuchFieldError
            let prev = thread().caught_exception.replace(Some((message, "JNI_GetStaticFieldID".to_string(), vm.null())));
            0 as jfieldID
        }
    }

    pub unsafe extern "system-unwind" fn GetStaticObjectField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jobject {
        let Value::Reference(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an object") };
        val.nid() as jobject
    }
    pub unsafe extern "system-unwind" fn GetStaticBooleanField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jboolean {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an integer") };
        val as jboolean
    }
    pub unsafe extern "system-unwind" fn GetStaticByteField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jbyte {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an integer") };
        val as jbyte
    }
    pub unsafe extern "system-unwind" fn GetStaticCharField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jchar {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an integer") };
        val as jchar
    }
    pub unsafe extern "system-unwind" fn GetStaticShortField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jshort {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an integer") };
        val as jshort
    }
    pub unsafe extern "system-unwind" fn GetStaticIntField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jint {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not an integer") };
        val as jint
    }
    pub unsafe extern "system-unwind" fn GetStaticLongField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jlong {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not a long") };
        val as jlong
    }
    pub unsafe extern "system-unwind" fn GetStaticFloatField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jfloat {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not a float") };
        val as jfloat
    }
    pub unsafe extern "system-unwind" fn GetStaticDoubleField(env: *mut JNIEnv, clazz: jclass, fieldID: jfieldID) -> jdouble {
        let Value::Integer(val) = (unsafe { Self::GetStaticField(env, clazz, fieldID) }) else { unimplemented!("GetStaticObjectField: not a double") };
        val as jdouble
    }

    pub fn SetStaticObjectField(clazz: jclass, fieldID: jfieldID, val: jobject){
        unimplemented!()
    }
    pub fn SetStaticBooleanField(clazz: jclass, fieldID: jfieldID, val: jboolean){
        unimplemented!()
    }
    pub fn SetStaticByteField(clazz: jclass, fieldID: jfieldID, val: jbyte){
        unimplemented!()
    }
    pub fn SetStaticCharField(clazz: jclass, fieldID: jfieldID, val: jchar){
        unimplemented!()
    }
    pub fn SetStaticShortField(clazz: jclass, fieldID: jfieldID, val: jshort){
        unimplemented!()
    }
    pub fn SetStaticIntField(clazz: jclass, fieldID: jfieldID, val: jint){
        unimplemented!()
    }
    pub fn SetStaticLongField(clazz: jclass, fieldID: jfieldID, val: jlong){
        unimplemented!()
    }
    pub fn SetStaticFloatField(clazz: jclass, fieldID: jfieldID, val: jfloat){
        unimplemented!()
    }
    pub fn SetStaticDoubleField(clazz: jclass, fieldID: jfieldID, val: jdouble){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn NewString(env: *mut JNIEnv, unicode: *const jchar, len: jsize) -> jstring{
        let raw_slice = unsafe { slice::from_raw_parts(unicode, len as usize) };
        let unicode_str = String::from_utf16_lossy(raw_slice);

        println!("NATIVE: NewString: '{}' , {:?}", unicode_str, raw_slice);
        let vm = unsafe{(*env).vm()};
        let str = vm.try_new_string_object(unicode_str.as_str()).map_err(|e| VmError::Native(e.to_string())).unwrap();
        str.id.nid()
    }
    pub unsafe extern "system-unwind" fn GetStringLength(env: *mut JNIEnv, str: jstring) -> jsize{
        let vm: &VM = unsafe{(*env).vm()};
        let string = vm.extract_string_from_value(Value::Reference(RefId(str))).unwrap();
        string.len() as jsize
    }
    pub unsafe extern "system-unwind" fn GetStringChars(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const jchar{
        let vm: &VM = unsafe{(*env).vm()};
        let string = vm.extract_string_from_value(Value::Reference(RefId(str))).unwrap();
        debug!(target: "native", "NATIVE: GetStringChars: {}", string);
        //FIXME potential dangling pointer
        let boxed_string = Box::new(string.encode_utf16().collect::<Vec<u16>>());
        Box::leak(boxed_string).as_ptr() as *const jchar
    }
    pub unsafe extern "system-unwind" fn ReleaseStringChars(env: *mut JNIEnv, str: jstring, chars: *const jchar){
        //nothing
    }
    //0x7ffde0618048
    //0x7ffde0618390
    pub unsafe extern "system-unwind" fn NewStringUTF(env: *const JNIEnv, utf: *const c_char) -> jstring{
        debug!("NATIVE: NewStringUTF");
        if utf.is_null() {
            return 0 as jstring
        }
        unsafe {
            let utf_r = CStr::from_ptr(utf).to_owned().into_string().map_err(|e| VmError::Native(e.to_string())).unwrap();
            let vm = (*env).vm();
            let str = vm.try_new_string_object(utf_r.as_str()).map_err(|e| VmError::Native(e.to_string())).unwrap();
            str.id.nid()
        }
    }
    pub unsafe extern "system-unwind" fn GetStringUTFLength(env: *mut JNIEnv, str: jstring) -> jsize{
        unimplemented!()
    }
    pub unsafe extern "system-unwind" fn GetStringUTFChars(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const c_char{
        unimplemented!()
    }
    pub unsafe extern "system-unwind" fn ReleaseStringUTFChars(env: *mut JNIEnv, str: jstring, chars: *const c_char){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetArrayLength(env: *mut JNIEnv, array: jarray) -> jsize{
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();
        array_ref.get_length() as jsize
    }

    pub unsafe extern "system-unwind" fn NewObjectArray(env: *mut JNIEnv, length: jsize, clazz: jclass, init: jobject) -> jobjectArray{
        let vm: &VM = unsafe{(*env).vm()};
        let clazz = vm.resolve_class_object_by_jclass(clazz);
        let init_val = if init != 0 {
            Value::Reference(RefId(init))
        } else {
            vm.null()
        };

        let content = vec![init_val; length as usize];
        let array_ref = native_init_wrap!(env, vm.new_array(
            1,
            FieldType::Object(clazz.name.clone()).to_array_field_type(1),
            RwLock::new(content.clone())
        ));
        array_ref.id.nid() as jobjectArray
    }

    pub unsafe extern "system-unwind" fn GetObjectArrayElement(env: *mut JNIEnv, array: jobjectArray, index: jsize) -> jobject {
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();
        if let Value::Reference(elem) = array_ref.get_element(index as usize) {
            elem.nid()
        } else {
            unreachable!("ObjectArray element is not an object")
        }
    }

    pub unsafe extern "system-unwind" fn SetObjectArrayElement(env: *mut JNIEnv, array: jarray, index: jsize, value: jobject){
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();
        let value_ref = if value != 0 {
            Value::Reference(RefId(value))
        } else {
            vm.null()
        };
        array_ref.set_element(index as usize, value_ref);
    }

    pub unsafe extern "system-unwind" fn NewByteArray(env: *mut JNIEnv, length: jsize) -> jbyteArray{
        let vm: &VM = unsafe{(*env).vm()};
        let content = vec![Value::Integer(0); length as usize];
        let array_ref = native_init_wrap!(env, vm.new_array(
            1,
            FieldType::Primitive(PrimitiveType::Byte).to_array_field_type(1),
            RwLock::new(content.clone())
        ));
        array_ref.id.nid() as jbyteArray
    }

    pub unsafe extern "system-unwind" fn GetByteArrayRegion(env: *mut JNIEnv, array: jbyteArray, start: jsize, len: jsize, buf: *mut jbyte){
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();
        if let ReferenceType::Array(_, _, content) = &array_ref.reference_type{
            content.read()
                .iter()
                .enumerate()
                .skip(start as usize)
                .for_each(|(i, val)| if let Value::Integer(b) = val {
                    unsafe {
                        *(buf.add(i)) = *b as jbyte
                    }
                } else {unreachable!()});
        } else {
            unimplemented!()
        }
    }

    pub unsafe extern "system-unwind" fn GetLongArrayRegion(env: *mut JNIEnv, array: jlongArray, start: jsize, len: jsize, buf: *mut jlong){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn SetByteArrayRegion(env: *mut JNIEnv, array: jbyteArray, start: jsize, len: jsize, buf: *mut jbyte) {
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();
        if let ReferenceType::Array(_, _, content) = &array_ref.reference_type{
            content.write()
                .iter_mut()
                .enumerate()
                .skip(start as usize)
                .for_each(|(i, val)| if let Value::Integer(b) = val {
                    unsafe {
                        *b = *(buf.add(i)) as i32
                    }
                } else {unreachable!()});
        } else {
            unimplemented!()
        }
    }

    pub unsafe extern "system-unwind" fn GetJavaVM(env: *mut JNIEnv, vm_ptr: *mut *const JavaVM) -> jint{
        unsafe {
            *vm_ptr = thread().java_vm.as_ref().get_ref() as *const JavaVM;
            println!("NATIVE: GetJavaVM: {:?} {:p}", *vm_ptr, *vm_ptr);
        }
        JNI_OK
    }

    pub unsafe extern "system-unwind" fn GetPrimitiveArrayCritical(env: *mut JNIEnv, array: jarray, isCopy: *mut jboolean) -> *const u8 {
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();

        if !isCopy.is_null() {
            unsafe {
                *isCopy = JNI_TRUE;
            }
        }

        if let ReferenceType::Array(_, ft, content) = &array_ref.reference_type {
            let width = ft.get_locals_length() * 4;
            let allocation_size = width * array_ref.get_length();
            let ptr = vm.unsafe_allocator.allocate_memory(allocation_size);

            let bytes: Vec<u8> = content.read()
                .iter()
                .enumerate()
                .flat_map(|(i, val)| match val {
                    Value::Integer(val) => val.to_le_bytes().to_vec(),
                    Value::Long(val) => val.to_le_bytes().to_vec(),
                    Value::Float(val) => val.to_le_bytes().to_vec(),
                    Value::Double(val) => val.to_le_bytes().to_vec(),
                    _ => unreachable!()
                })
                .collect();
            vm.unsafe_allocator.put_bytes(ptr, &bytes).unwrap();

            ptr as *const u8
        } else {
            ptr::null()
        }
    }

    pub unsafe extern "system-unwind" fn ReleasePrimitiveArrayCritical(env: *mut JNIEnv, array: jarray, carray: *const u8, mode: jint) {
        let vm: &VM = unsafe{(*env).vm()};
        let array_ref = vm.resolve_object_by_jobject(array).unwrap();

        if let ReferenceType::Array(_, ft, content) = &array_ref.reference_type {
            let width = ft.get_locals_length() * 4;
            let allocation_size = width * array_ref.get_length();

            let bytes = vm.unsafe_allocator.get_bytes(carray as i64, allocation_size).unwrap();
            let vals= bytes.chunks(width).map(|bytes| match ft {
                FieldType::Primitive(PrimitiveType::Boolean) => Value::Integer(i32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Byte) => Value::Integer(i32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Char) => Value::Integer(i32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Short) => Value::Integer(i32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Integer) => Value::Integer(i32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Long) => Value::Long(i64::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Float) => Value::Float(f32::from_le_bytes(bytes.try_into().unwrap())),
                FieldType::Primitive(PrimitiveType::Double) => Value::Double(f64::from_le_bytes(bytes.try_into().unwrap())),
                _ => unreachable!()
            }).collect::<Vec<_>>();

            let mut guard = content.write();
            guard.copy_from_slice(vals.as_slice())
        }
    }

    pub unsafe extern "system-unwind" fn ExceptionCheck(env: *mut JNIEnv) -> jboolean{
        let vm: &VM = unsafe{(*env).vm()};
        if thread().caught_exception.borrow().is_some() {JNI_TRUE} else {JNI_FALSE}
    }

    pub unsafe extern "system-unwind" fn NewDirectByteBuffer(env: *mut JNIEnv, address: *const c_void, capacity: jlong) -> jobject{
        unimplemented!()
    }

}

#[repr(C)]
struct JVMOption{
    optionString: *const c_char,
    extraInfos:   *const c_void,
}

#[repr(C)]
struct JavaVMInitArgs{
    version:            jint,
    
    nOptions:           jint,
    options:            *mut JVMOption,
    ignoreUnrecognized: jboolean,
}

struct JavaVMAttachArgs{
    version:            jint,
    
    name:               *const c_char,
    group:              jobject,
}

#[repr(C)]
pub struct JNIInvokeInterface_{}

impl JNIInvokeInterface_ {
    pub fn DestroyJavaVM(vm: *mut JavaVM) -> jint{
        unimplemented!()
    }
    pub fn AttachCurrentThread(vm: *mut JavaVM, penv: *const *const c_void, args: *const c_void) -> jint{
        unimplemented!()
    }
    pub fn DetachCurrentThread(vm: *mut JavaVM) -> jint{
        unimplemented!()
    }
    pub unsafe extern "system-unwind" fn GetEnv(vm: *mut JavaVM, penv: *mut *const c_void, version: jint) -> jint{
        unsafe {
            *penv = thread().jni_env.as_ref().get_ref() as *const JNIEnv as _;
            println!("{:?} {:p}", *penv, *penv);
        }
        JNI_OK
    }
    pub fn AttachCurrentThreadAsDaemon(vm: *mut JavaVM, penv: *const *const c_void, args: *const c_void) -> jint{
        unimplemented!()
    }
}

pub const JNI_VERSION_1_1: i32 = 0x00010001;
pub const JNI_VERSION_1_2: i32 = 0x00010002;
pub const JNI_VERSION_1_4: i32 = 0x00010004;
pub const JNI_VERSION_1_6: i32 = 0x00010006;
pub const JNI_VERSION_1_8: i32 = 0x00010008;
