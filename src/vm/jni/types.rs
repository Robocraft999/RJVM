#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use std::ffi::{c_char, c_double, c_float, c_int, c_long, c_schar, c_short, c_uchar, c_ushort, c_void, CStr};
use log::error;
use crate::vm::{jni::{env_function_table::JNINativeInterface, vm_function_table::JNIInvokeInterface}, VM};

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
pub type jobject = *mut _jobject;
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
    z: jboolean,
    b: jbyte,
    c: jchar,
    s: jshort,
    i: jint,
    j: jlong,
    f: jfloat,
    d: jdouble,
    l: jobject,
}

#[repr(C)]
pub struct _jfieldID;
pub type jfieldID = *mut _jfieldID;
#[repr(C)]
pub struct _jmethodID;
pub type jmethodID = *mut _jmethodID;

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
}

/*
 * JNI Invocation Interface.
 */

#[repr(C)]
pub struct JavaVM<'a>{
    pub methods: *const JNIInvokeInterface,
    pub env: JNIEnv<'a>,
}

#[repr(C)]
pub struct JNINativeInterface_{}

impl JNINativeInterface_ {
    pub fn GetVersion(env: *mut JNIEnv) -> jint{
        unimplemented!()
    }

    pub fn DefineClass(env: *mut JNIEnv, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize) -> jclass {
        unimplemented!()
    }
    pub fn FindClass(env: *mut JNIEnv, name: *const c_char) -> jclass {
        let name = unsafe{CStr::from_ptr(name)};
        println!("{:?}", name);
        if name.to_string_lossy() == ""{
            0 as jclass
        } else {
            unimplemented!()
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
        unimplemented!()
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
        unimplemented!()
    }
    pub fn DeleteGlobalRef(env: *mut JNIEnv, gref: jobject){
        unimplemented!()
    }
    pub fn DeleteLocalRef(env: *mut JNIEnv, obj: jobject){
        unimplemented!()
    }
    pub fn IsSameObject(env: *mut JNIEnv, obj1: jobject, obj2: jobject) -> jboolean{
        unimplemented!()
    }
    pub fn NewLocalRef(env: *mut JNIEnv, r#ref: jobject) -> jobject{
        unimplemented!()
    }
    pub fn EnsureLocalCapacity(env: *mut JNIEnv, capacity: jint) -> jint{
        unimplemented!()
    }

    pub fn AllocObject(env: *mut JNIEnv, clazz: jclass) -> jobject{
        unimplemented!()
    }
    pub fn NewObjectA(env: *mut JNIEnv, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }

    pub fn GetObjectClass(obj: jobject) -> jclass{
        unimplemented!()
    }
    pub fn IsInstanceOf(obj: jobject, clazz: jclass){
        unimplemented!()
    }

    pub fn GetMethodID(clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        unimplemented!()
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

    pub fn GetFieldID(clazz: jclass, name: *const c_char, sig: *const c_char) -> jfieldID{
        unimplemented!()
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

    pub fn GetStaticMethodID(clazz: jclass, name: *const c_char, sig: *const c_char) -> jmethodID{
        unimplemented!()
    }

    pub fn CallStaticObjectMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jobject{
        unimplemented!()
    }
    pub fn CallStaticBooleanMethodA(clazz: jclass, methodID: jmethodID, args: *const jvalue) -> jboolean{
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

    pub fn NewString(unicode: *const c_char, len: jsize) -> jstring{
        unimplemented!()
    }
    pub fn GetStringLength(str: jstring) -> jsize{
        unimplemented!()
    }
    pub fn GetStringChars(str: jstring, isCopy: *mut jboolean) -> *const jchar{
        unimplemented!()
    }
    pub fn ReleaseStringChars(str: jstring, chars: *const jchar){
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
        (*penv) = &(*vm).env as *const JNIEnv as _;
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
