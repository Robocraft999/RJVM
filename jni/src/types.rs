#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use std::ffi::{c_char, c_double, c_float, c_int, c_long, c_schar, c_short, c_uchar, c_ushort, c_void};

//Platform dependent
type jint = c_int;
type jlong = c_long;
type jbyte = c_schar;

//Primitives
type jboolean = c_uchar;
type jchar = c_ushort;
type jshort = c_short;
type jfloat = c_float;
type jdouble = c_double;
type jsize = jint;

//Object related
#[repr(C)]
struct _jobject;
type jobject = *mut _jobject;
type jclass = jobject;
type jthrowable = jobject;
type jstring = jobject;
type jarray = jobject;
type jbooleanArray = jarray;
type jbyteArray = jarray;
type jcharArray = jarray;
type jshortArray = jarray;
type jintArray = jarray;
type jlongArray = jarray;
type jfloatArray = jarray;
type jdoubleArray = jarray;
type jobjectArray = jarray;

//Value
type jweak = jobject;

#[repr(C)]
union jvalue{
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
struct _jfieldID;
type jfieldID = *mut _jfieldID;
#[repr(C)]
struct _jmethodID;
type jmethodID = *mut _jmethodID;

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
struct JNINativeMethod{
    name:      *const c_char,
    signature: *const c_char,
    fnPtr:     *const c_void,
}

/*
 * JNI Native Method Interface.
 */

type JNIEnv = *mut JNINativeInterface_;

/*
 * JNI Invocation Interface.
 */

type JavaVM = *mut JNIInvokeInterface_;

struct JNINativeInterface_{
    reserved0: *const c_void,
    reserved1: *const c_void,
    reserved2: *const c_void,
    reserved3: *const c_void,
}

impl JNINativeInterface_ {
    pub fn GetVersion(&mut self) -> jint{
        unimplemented!()
    }

    pub fn DefineClass(&mut self, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize) -> jclass {
        unimplemented!()
    }
    pub fn FindClass(&mut self, name: *const c_char) -> jclass {
        unimplemented!()
    }

    pub fn FromReflectedMethod(&mut self, method: jobject) -> jmethodID {
        unimplemented!()
    }
    pub fn FromReflectedField(&mut self, field: jobject) -> jfieldID {
        unimplemented!()
    }

    pub fn ToReflectedMethod(&mut self, cls: jclass, method: jmethodID, isStatic: jboolean) -> jobject {
        unimplemented!()
    }

    pub fn GetSuperclass(&mut self, sub: jclass) -> jclass{
        unimplemented!()
    }
    pub fn IsAssignableFrom(&mut self, sub: jclass, sup: jclass) -> jboolean {
        unimplemented!()
    }

    pub fn ToReflectedField(&mut self, cls: jclass, field: jfieldID, isStatic: jboolean) -> jobject {
        unimplemented!()
    }

    pub fn Throw(&mut self, obj: jthrowable) -> jint{
        unimplemented!()
    }
    pub fn ThrowNew(&mut self, clazz: jclass, msg: *const c_char) -> jint{
        unimplemented!()
    }
    pub fn ExceptionOccurred(&mut self) -> jthrowable{
        unimplemented!()
    }
    pub fn ExceptionDescribe(&mut self){
        unimplemented!()
    }
    pub fn ExceptionClear(&mut self){
        unimplemented!()
    }
    pub fn FatalError(&mut self, msg: *const c_char){
        unimplemented!()
    }

    pub fn PushLocalFrame(&mut self, capacity: jint) -> jint{
        unimplemented!()
    }
    pub fn PopLocalFrame(&mut self, result: jobject) -> jint{
        unimplemented!()
    }

    pub fn NewGlobalRef(&mut self, lobj: jobject) -> jobject{
        unimplemented!()
    }
    pub fn DeleteGlobalRef(&mut self, gref: jobject){
        unimplemented!()
    }
    pub fn DeleteLocalRef(&mut self, obj: jobject){
        unimplemented!()
    }
    pub fn IsSameObject(&mut self, obj1: jobject, obj2: jobject) -> jboolean{
        unimplemented!()
    }
    pub fn NewLocalRef(&mut self, r#ref: jobject) -> jobject{
        unimplemented!()
    }
    pub fn EnsureLocalCapacity(&mut self, capacity: jint) -> jint{
        unimplemented!()
    }

    pub fn AllocObject(&mut self, clazz: jclass) -> jobject{
        unimplemented!()
    }
    pub fn NewObjectA(&mut self, methodID: jmethodID, args: *const jvalue) -> jobject{
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
struct JNIInvokeInterface_{
    reserved0: *const c_void,
    reserved1: *const c_void,
    reserved2: *const c_void,
}

impl JNIInvokeInterface_ {
    pub fn DestroyJavaVM(&mut self) -> jint{
        unimplemented!()
    }
    pub fn AttachCurrentThread(&mut self, penv: *const *const c_void, args: *const c_void) -> jint{
        unimplemented!()
    }
    pub fn DetachCurrentThread(&mut self) -> jint{
        unimplemented!()
    }
    pub fn GetEnv(&mut self, penv: *const *const c_void, version: jint) -> jint{
        unimplemented!()
    }
    pub fn AttachCurrentThreadAsDaemon(&mut self, penv: *const *const c_void, args: *const c_void) -> jint{
        unimplemented!()
    }
}
