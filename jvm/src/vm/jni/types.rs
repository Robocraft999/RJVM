#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use crate::vm::{jni::{env_function_table::JNINativeInterface, vm_function_table::JNIInvokeInterface}, VmError, VM};
use log::{debug, error};
use std::ffi::{c_char, c_double, c_float, c_int, c_long, c_schar, c_short, c_uchar, c_ushort, c_void, CStr, CString, OsStr, VaList};
use std::fmt::Debug;
use std::os::unix::ffi::OsStrExt;
use std::slice;
use crate::field_info::{FieldType, PrimitiveType};
use crate::vm::class::{ClassAndMethod, ClassId};
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::Value;

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
pub struct JNIEnv<'a>{
    pub(crate) methods: *const JNINativeInterface,
    pub vm: *const VM<'a>,
    pub pvm: *const JavaVM<'a>,
}

/*
 * JNI Invocation Interface.
 */

#[repr(C)]
pub struct JavaVM<'a>{
    pub methods: *const JNIInvokeInterface,
    pub env: JNIEnv<'a>,
}

impl !Unpin for JavaVM<'_>{}

#[repr(C)]
pub struct JNINativeInterface_{}

macro_rules! wrap_init{
    ($env:expr, $x:expr) => {
        {
            let macro_vm: &VM = unsafe{&*(*$env).vm};
            let macro_javavm: &JavaVM = unsafe{&*(*$env).pvm};
            let macro_current_frame_index: isize = macro_vm.call_stack.len() as isize -1;
            let macro_res = $x;
            match macro_res.unwrap(){
                VMResultType::Successful(v) => v,
                VMResultType::Interrupted(..) => {
                    let init_res = macro_vm.invoke_frames_until(macro_javavm, macro_current_frame_index).unwrap();
                    if let VMResultType::Successful(None) = init_res{
                        if let VMResultType::Successful(v) = ($x).unwrap(){
                            v
                        } else {
                            unreachable!("[wrap_init] still needs classes even after loading them")
                        }
                    } else {
                        unreachable!("[wrap_init] still classes to init after initting them")
                    }
                }
                other => unreachable!("[wrap_init] got unexpected: {:?}", other),
            }
        }
    }
}

unsafe fn resolve_static_class_and_method(env: *mut JNIEnv, clazz: jclass, method_id: jmethodID) -> ClassAndMethod{
    let vm = unsafe{&*(*env).vm};

    let class_obj = vm.objects_by_id.borrow().get(&clazz).copied().unwrap();
    let class_ref = wrap_init!(env, vm.extract_class_from_class_object(&class_obj));

    let method_info = class_ref.methods.get(method_id).unwrap();
    ClassAndMethod{class: class_ref, method: method_info}
}

unsafe fn resolve_class_and_method(env: *mut JNIEnv, obj: jobject, method_id: jmethodID) -> ClassAndMethod{
    let vm = unsafe{&*(*env).vm};

    let obj_ref = vm.objects_by_id.borrow().get(&obj).copied().unwrap();
    let class_ref = wrap_init!(env, vm.get_or_resolve_class(obj_ref.class_name.as_str()));

    let method_info = class_ref.methods.get(method_id).unwrap();
    ClassAndMethod{class: class_ref, method: method_info}
}

unsafe fn resolve_function_args<'a>(env: *mut JNIEnv<'a>, class_and_method: &ClassAndMethod, mut raw: VaList) -> Vec<Value<'a>>{
    let vm = unsafe{&*(*env).vm};
    class_and_method.method.descriptor.args.iter().flat_map(|ft| match ft{
        FieldType::Object(..) | FieldType::Array(..) => {
            let ref_id: u32 = unsafe{raw.arg()};
            {
                let reference = vm.objects_by_id.borrow().get(&ref_id).copied().unwrap();
                println!("NATIVE: arg for {}: {:?}", class_and_method.format(), reference);
                vec![Value::Reference(reference)]
            }
        }
        FieldType::Primitive(pt) => match pt{
            PrimitiveType::Boolean | PrimitiveType::Byte | PrimitiveType::Char | PrimitiveType::Integer | PrimitiveType::Short => {
                let i: i32 = unsafe{raw.arg()};
                vec![Value::Integer(i)]
            }
            PrimitiveType::Float => {
                let f: f64 = unsafe{raw.arg()};
                vec![Value::Float(f as f32)]
            }
            PrimitiveType::Double => {
                let d: f64 = unsafe{raw.arg()};
                vec![Value::Double(d), Value::Dummy]
            }
            PrimitiveType::Long => {
                let l: i64 = unsafe{raw.arg()};
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
        println!("NATIVE: FindClass: '{}'", name);
        if name == ""{
            0 as jclass
        } else {
            let vm = unsafe{&*(*env).vm};
            let class = if let Some(class) = vm.find_class_by_name(name){
                class
            } else {
                wrap_init!(env, vm.get_or_resolve_class(name))
            };
            let class_obj = wrap_init!(env, vm.new_class_object_by_class(class));
            class_obj.id
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

    pub fn Throw(env: *mut JNIEnv, obj: jthrowable) -> jint{
        unimplemented!()
    }
    pub fn ThrowNew(env: *mut JNIEnv, clazz: jclass, msg: *const c_char) -> jint{
        unimplemented!()
    }
    pub fn ExceptionOccurred(env: *mut JNIEnv) -> jthrowable{
        let vm: &VM = unsafe{&*(*env).vm};
        if let Some((_, _, Value::Reference(throwable))) = &vm.caught_exception.borrow().as_ref(){
            throwable.id as jthrowable
        } else {
            0 as jthrowable
        }
    }
    pub fn ExceptionDescribe(env: *mut JNIEnv){
        unimplemented!()
    }
    pub fn ExceptionClear(env: *mut JNIEnv){
        unimplemented!()
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
        let vm = unsafe{&*(*env).vm};
        debug!("NATIVE: NewGlobalRef: {:?}", vm.objects_by_id.borrow().get(&lobj));
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
    pub fn NewObjectA(env: *mut JNIEnv, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetObjectClass(env: *mut JNIEnv, obj: jobject) -> jclass{
        let vm: &VM = unsafe{&*(*env).vm};
        if let Some(object) = vm.objects_by_id.borrow().get(&obj){
            let class_obj = wrap_init!(env, vm.new_class_object(object.class_name.as_str(), object.class_id));
            class_obj.id as jclass
        } else {
            0 as jclass
        }

    }
    pub fn IsInstanceOf(obj: jobject, clazz: jclass){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetMethodID(env: *mut JNIEnv, clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        let method_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{&*(*env).vm};

        let class_obj = vm.objects_by_id.borrow().get(&clazz).copied().unwrap();
        let class_ref = wrap_init!(env, vm.extract_class_from_class_object(&class_obj));
        println!("NATIVE: GetMethodID: {}::{}{}", class_ref.name, method_name, signature);
        //FIXME zero index results in NULL
        class_ref.find_method_index(method_name, signature).ok_or(VmError::Native(format!("GetMethodID: {}::{}{} not found", class_ref.name, method_name, signature))).unwrap()
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
        unsafe{Self::CallVoidMethodV(env, obj, methodID, params.as_va_list())}
    }

    pub unsafe extern "system-unwind" fn CallVoidMethodV(env: *mut JNIEnv, obj: jobject, methodID: jmethodID, args: VaList){
        let vm = unsafe{&*(*env).vm};
        let class_and_method = unsafe{ resolve_class_and_method(env, obj, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let stop_index = vm.call_stack.len() as isize -1;
        let javavm = unsafe{&*(*env).pvm};

        let obj_ref = vm.objects_by_id.borrow().get(&obj).copied().unwrap();
        debug!("CallVoidMethodV: {} ({:?})", class_and_method.format(), args);
        vm.call_stack.create_and_push_call_frame(class_and_method, Some(obj_ref), args, false);
        let res = vm.invoke_frames_until(javavm, stop_index).unwrap();
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
        let vm = unsafe{&*(*env).vm};

        let class_obj = vm.objects_by_id.borrow().get(&clazz).copied().unwrap();
        let class_ref = wrap_init!(env, vm.extract_class_from_class_object(&class_obj));
        println!("NATIVE: GetFieldID: {}::{}{}", class_ref.name, field_name, signature);
        // FIXME same as GetMethodID, there is a field at index 0 which is recognized as NULL
        if let Some((index, _)) = class_ref.find_field(field_name){
            index as jfieldID
        } else {
            let message = format!("GetMethodID: {}::{} not found", class_ref.name, field_name);
            // NoSuchFieldError
            let prev = vm.caught_exception.replace(Some((message, "JNI_GetFieldID".to_string(), vm.null())));
            0 as jfieldID
        }
    }

    pub fn GetObjectField(obj: jobject, fieldID: jfieldID) -> jobject{
        unimplemented!()
    }
    pub fn GetBooleanField(obj: jobject, fieldID: jfieldID) -> jboolean{
        unimplemented!()
    }
    pub fn GetByteField(obj: jobject, fieldID: jfieldID) -> jbyte{
        unimplemented!()
    }
    pub fn GetCharField(obj: jobject, fieldID: jfieldID) -> jchar{
        unimplemented!()
    }
    pub fn GetShortField(obj: jobject, fieldID: jfieldID) -> jshort{
        unimplemented!()
    }
    pub fn GetIntField(obj: jobject, fieldID: jfieldID) -> jint{
        unimplemented!()
    }
    pub fn GetLongField(obj: jobject, fieldID: jfieldID) -> jlong{
        unimplemented!()
    }
    pub fn GetFloatField(obj: jobject, fieldID: jfieldID) -> jfloat{
        unimplemented!()
    }
    pub fn GetDoubleField(obj: jobject, fieldID: jfieldID) -> jdouble{
        unimplemented!()
    }

    pub fn SetObjectField(obj: jobject, fieldID: jfieldID, val: jobject){
        unimplemented!()
    }
    pub fn SetBooleanField(obj: jobject, fieldID: jfieldID, val: jboolean){
        unimplemented!()
    }
    pub fn SetByteField(obj: jobject, fieldID: jfieldID, val: jbyte){
        unimplemented!()
    }
    pub fn SetCharField(obj: jobject, fieldID: jfieldID, val: jchar){
        unimplemented!()
    }
    pub fn SetShortField(obj: jobject, fieldID: jfieldID, val: jshort){
        unimplemented!()
    }
    pub fn SetIntField(obj: jobject, fieldID: jfieldID, val: jint){
        unimplemented!()
    }
    pub fn SetLongField(obj: jobject, fieldID: jfieldID, val: jlong){
        unimplemented!()
    }
    pub fn SetFloatField(obj: jobject, fieldID: jfieldID, val: jfloat){
        unimplemented!()
    }
    pub fn SetDoubleField(obj: jobject, fieldID: jfieldID, val: jdouble){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetStaticMethodID(env: *mut JNIEnv, clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        let method_name = unsafe{CStr::from_ptr(name)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let signature = unsafe{CStr::from_ptr(sig)}.to_str().map_err(|e| VmError::Native(e.to_string())).unwrap();
        let vm = unsafe{&*(*env).vm};

        let class_obj = vm.objects_by_id.borrow().get(&clazz).copied().unwrap();
        let class_ref = wrap_init!(env, vm.extract_class_from_class_object(&class_obj));
        println!("NATIVE: GetStaticMethodID: {}::{}{}", class_ref.name, method_name, signature);
        //FIXME zero index results in NULL
        class_ref.find_method_index(method_name, signature).ok_or(VmError::Native(format!("GetStaticMethodID: {}::{}{} not found", class_ref.name, method_name, signature))).unwrap()
    }

    pub unsafe extern "system-unwind" fn CallStaticObjectMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, mut args: VaList) -> jobject{
        let vm = unsafe{&*(*env).vm};

        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let stop_index = vm.call_stack.len() as isize -1;
        let javavm = unsafe{&*(*env).pvm};

        debug!("CallStaticObjectMethodV: {} ({:?})", class_and_method.format(), args);
        vm.call_stack.create_and_push_call_frame(class_and_method, None, args, false);
        let res = vm.invoke_frames_until(javavm, stop_index).unwrap();
        if let VMResultType::Successful(Some(result)) = res{
            if let Value::Reference(r) = result{
                r.id as jobject
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

    pub unsafe extern "system-unwind" fn CallStaticBooleanMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: VaList) -> jboolean{
        let vm = unsafe{&*(*env).vm};
        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let stop_index = vm.call_stack.len() as isize -1;
        let javavm = unsafe{&*(*env).pvm};

        debug!("CallStaticBooleanMethod: {} ({:?})", class_and_method.format(), args);
        vm.call_stack.create_and_push_call_frame(class_and_method, None, args, false);
        let res = vm.invoke_frames_until(javavm, stop_index).unwrap();
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
    pub unsafe extern "system-unwind" fn CallStaticVoidMethodV(env: *mut JNIEnv, clazz: jclass, methodID: jmethodID, args: VaList){
        let vm = unsafe{&*(*env).vm};
        let class_and_method = unsafe{ resolve_static_class_and_method(env, clazz, methodID)};
        let args = unsafe{resolve_function_args(env, &class_and_method, args)};

        let stop_index = vm.call_stack.len() as isize -1;
        let javavm = unsafe{&*(*env).pvm};

        debug!("CallStaticVoidMethod: {} ({:?})", class_and_method.format(), args);
        vm.call_stack.create_and_push_call_frame(class_and_method, None, args, false);
        let res = vm.invoke_frames_until(javavm, stop_index).unwrap();
        if let VMResultType::Successful(None) = res{
            //works
        } else {
            unimplemented!("CallStaticVoidMethod: expected no return value but got: {:?}", res)
        }
    }
    pub fn CallStaticVoidMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue){
        unimplemented!()
    }

    pub fn GetStaticFieldID(clazz: jclass, name: *const c_char, sig: *const c_char) -> jfieldID{
        unimplemented!()
    }

    pub fn GetStaticObjectField(clazz: jclass, fieldID: jfieldID) -> jobject{
        unimplemented!()
    }
    pub fn GetStaticBooleanField(clazz: jclass, fieldID: jfieldID) -> jboolean{
        unimplemented!()
    }
    pub fn GetStaticByteField(clazz: jclass, fieldID: jfieldID) -> jbyte{
        unimplemented!()
    }
    pub fn GetStaticCharField(clazz: jclass, fieldID: jfieldID) -> jchar{
        unimplemented!()
    }
    pub fn GetStaticShortField(clazz: jclass, fieldID: jfieldID) -> jshort{
        unimplemented!()
    }
    pub fn GetStaticIntField(clazz: jclass, fieldID: jfieldID) -> jint{
        unimplemented!()
    }
    pub fn GetStaticLongField(clazz: jclass, fieldID: jfieldID) -> jlong{
        unimplemented!()
    }
    pub fn GetStaticFloatField(clazz: jclass, fieldID: jfieldID) -> jfloat{
        unimplemented!()
    }
    pub fn GetStaticDoubleField(clazz: jclass, fieldID: jfieldID) -> jdouble{
        unimplemented!()
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

    pub unsafe extern "system" fn NewString(env: *mut JNIEnv, unicode: *const jchar, len: jsize) -> jstring{
        let raw_slice = unsafe { slice::from_raw_parts(unicode, len as usize) };
        let unicode_str = String::from_utf16_lossy(raw_slice);

        println!("NATIVE: NewString: '{}' , {:?}", unicode_str, raw_slice);
        let vm = unsafe{&*(*env).vm};
        let str = vm.try_new_string_object(unicode_str.as_str()).map_err(|e| VmError::Native(e.to_string())).unwrap();
        str.id
    }
    pub fn GetStringLength(env: *mut JNIEnv, str: jstring) -> jsize{
        unimplemented!()
    }
    pub fn GetStringChars(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const jchar{
        unimplemented!()
    }
    pub fn ReleaseStringChars(env: *mut JNIEnv, str: jstring, chars: *const jchar){
        unimplemented!()
    }
    //0x7ffde0618048
    //0x7ffde0618390
    pub unsafe extern "system" fn NewStringUTF(env: *const JNIEnv, utf: *const c_char) -> jstring{
        debug!("NATIVE: NewStringUTF");
        unsafe {
            let utf_r = CStr::from_ptr(utf).to_owned().into_string().map_err(|e| VmError::Native(e.to_string())).unwrap();
            let vm = &*(*env).vm;
            let str = vm.try_new_string_object(utf_r.as_str()).map_err(|e| VmError::Native(e.to_string())).unwrap();
            str.id
        }
    }
    pub unsafe extern "system" fn GetStringUTFLength(env: *mut JNIEnv, str: jstring) -> jsize{
        unimplemented!()
    }
    pub unsafe extern "system" fn GetStringUTFChars(env: *mut JNIEnv, str: jstring, isCopy: *mut jboolean) -> *const c_char{
        unimplemented!()
    }
    pub unsafe extern "system" fn ReleaseStringUTFChars(env: *mut JNIEnv, str: jstring, chars: *const c_char){
        unimplemented!()
    }

    pub unsafe extern "system" fn GetArrayLength(env: *mut JNIEnv, array: jarray){
        unimplemented!()
    }

    pub unsafe extern "system" fn GetLongArrayRegion(env: *mut JNIEnv, array: jlongArray, start: jsize, len: jsize, buf: *mut jlong){
        unimplemented!()
    }

    pub unsafe extern "system-unwind" fn GetJavaVM(env: *mut JNIEnv, vm_ptr: *mut *const JavaVM) -> jint{
        unsafe {
            *vm_ptr = (*env).pvm as _;
            println!("NATIVE: GetJavaVM: {:?} {:p}", *vm_ptr, *vm_ptr);
        }
        JNI_OK
    }

    pub unsafe extern "system-unwind" fn ExceptionCheck(env: *mut JNIEnv) -> jboolean{
        let vm: &VM = unsafe{&*(*env).vm};
        if vm.caught_exception.borrow().is_some() {JNI_TRUE} else {JNI_FALSE}
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
    pub unsafe extern "system" fn GetEnv(vm: *mut JavaVM, penv: *mut *const c_void, version: jint) -> jint{
        unsafe {
            *penv = &(*vm).env as *const JNIEnv as _;
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
