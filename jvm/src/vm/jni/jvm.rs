#![allow(unused_variables)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(private_interfaces)]

use crate::class_file::fields::field_type::FieldType;
use crate::native_init_wrap;
use crate::vm::jni::types::*;
use crate::vm::value::ReferenceType;
use crate::vm::{VMResultType, VM};
use log::debug;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_long, c_uchar, c_ushort, c_void, CStr, CString};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::str::FromStr;

pub const JVM_INTERFACE_VERSION: jint = 4;

#[repr(C)]
struct jvm_version_info{
    jvm_version: u32,
    update_version: u32,
    special_update_version: u32,
    reserved1: u32,
    reserved2: u32,
    is_attachable: u32,
    unnamed0: u32,
    unnamed1: u32,
    unnamed2: u32,
}

#[repr(C)]
struct sockaddr;

#[repr(C)]
struct JVM_DTraceProbe{
    method: jmethodID,
    function: jstring,
    name: jstring,
    reserved: [*const c_void; 4],
}

#[repr(C)]
struct JVM_DTraceInterfaceAttributes{
    nameStability: jint,
    dataStability: jint,
    dependencyClass: jint,
}

#[repr(C)]
struct JVM_DTraceProvider{
    name: jstring,
    probes: *const JVM_DTraceProbe,
    probe_count: jint,
    providerAttributes: JVM_DTraceInterfaceAttributes,
    moduleAttributes: JVM_DTraceInterfaceAttributes,
    functionAttributes: JVM_DTraceInterfaceAttributes,
    nameAttributes: JVM_DTraceInterfaceAttributes,
    argsAttributes: JVM_DTraceInterfaceAttributes,
    reserved: [*const c_void; 4],
}

#[repr(C)]
struct JVM_ExceptionTableEntryType{
    start_pc: jint,
    end_pc: jint,
    handler_pc: jint,
    catchType: jint,
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodParameters(env: *mut JNIEnv, method: jobject) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetInterfaceVersion() -> jint {
    JVM_INTERFACE_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IHashCode(env: *mut JNIEnv, obj: jobject) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_MonitorWait(env: *mut JNIEnv, obj: jobject, ms: jlong) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_MonitorNotify(env: *mut JNIEnv, obj: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_MonitorNotifyAll(env: *mut JNIEnv, obj: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Clone(env: *mut JNIEnv, obj: jobject) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InternString(env: *mut JNIEnv, str: jstring) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CurrentTimeMillis(env: *mut JNIEnv, ignored: jclass) -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_NanoTime(env: *mut JNIEnv, ignored: jclass) -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ArrayCopy(env: *mut JNIEnv, ignored: jclass, src: jobject, src_pos: jint, dst: jobject, dst_pos: jint, length: jint) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InitProperties(env: *mut JNIEnv, p: jobject) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_OnExit() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CopySwapMemory(env: *mut JNIEnv, srcObj: jobject, srcOffset: jlong, dstObj: jobject, dstOffset: jlong, size: jlong, elemSize: jlong) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_BeforeHalt() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Halt(code: jint) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GC() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_MaxObjectInspectionAge() -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_TraceInstructions(on: jboolean) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_TraceMethodCalls(on: jboolean) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_TotalMemory() -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FreeMemory() -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_MaxMemory() -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ActiveProcessorCount() -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsUseContainerSupport() -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_LoadLibrary(name: *const c_char) -> *const c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_UnloadLibrary(handle: *const c_void) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindLibraryEntry(handle: *const c_void, name: *const c_char) -> *const c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsSupportedJNIVersion(version: jint) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsNaN(d: jdouble) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FillInStackTrace(env: *mut JNIEnv, throwable: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetStackTraceDepth(env: *mut JNIEnv, throwable: jobject) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetStackTraceElement(env: *mut JNIEnv, throwable: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InitializeCompiler(env: *mut JNIEnv, compCls: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsSilentCompiler(env: *mut JNIEnv, compCls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CompileClass(env: *mut JNIEnv, compCls: jclass, cls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CompileClasses(env: *mut JNIEnv, cls: jclass, jname: jstring) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CompilerCommand(env: *mut JNIEnv, compCls: jclass, arg: jobject) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_EnableCompiler(env: *mut JNIEnv, compCls: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DisableCompiler(env: *mut JNIEnv, compCls: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_StartThread(env: *mut JNIEnv, thread: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_StopThread(env: *mut JNIEnv, thread: jobject, exception: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsThreadAlive(env: *mut JNIEnv, thread: jobject) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SuspendThread(env: *mut JNIEnv, thread: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ResumeThread(env: *mut JNIEnv, thread: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetThreadPriority(env: *mut JNIEnv, thread: jobject, prio: jint) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Yield(env: *mut JNIEnv, threadClass: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Sleep(env: *mut JNIEnv, threadClass: jclass, millis: jlong) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CurrentThread(env: *mut JNIEnv, threadClass: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CountStackFrames(env: *mut JNIEnv, thread: jobject) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Interrupt(env: *mut JNIEnv, thread: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsInterrupted(env: *mut JNIEnv, thread: jobject, clearInterrupted: jboolean) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_HoldsLock(env: *mut JNIEnv, threadClass: jclass, obj: jobject) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DumpAllStacks(env: *mut JNIEnv, unused: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetAllThreads(env: *mut JNIEnv, dummy: jclass) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetNativeThreadName(env: *mut JNIEnv, jthread: jobject, name: jstring) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DumpThreads(env: *mut JNIEnv, threadClass: jclass, threads: jobjectArray) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CurrentLoadedClass(env: *mut JNIEnv) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CurrentClassLoader(env: *mut JNIEnv) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassContext(env: *mut JNIEnv) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ClassDepth(env: *mut JNIEnv, name: jstring) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ClassLoaderDepth(env: *mut JNIEnv) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetSystemPackage(env: *mut JNIEnv, name: jstring) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetSystemPackages(env: *mut JNIEnv) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_AllocateNewObject(env: *mut JNIEnv, obj: jobject, currClass: jclass, initClass: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_AllocateNewArray(env: *mut JNIEnv, obj: jobject, currClass: jclass, length: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_LatestUserDefinedLoader(env: *mut JNIEnv) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_LoadClass0(env: *mut JNIEnv, obj: jobject, currClass: jclass, currClassName: jstring) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetArrayLength(env: *mut JNIEnv, arr: jobject) -> jint {
    let vm = unsafe{&*(*env).vm};

    let obj_ref = vm.resolve_object_by_jobject(arr).unwrap();
    if let ReferenceType::Array(_, _, content) = &obj_ref.reference_type {
        content.borrow().len() as jint
    } else {
        unreachable!("fixme error handling")
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetArrayElement(env: *mut JNIEnv, arr: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetPrimitiveArrayElement(env: *mut JNIEnv, arr: jobject, index: jint, wCode: jint) -> jvalue {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetArrayElement(env: *mut JNIEnv, arr: jobject, index: jint, val: jobject) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetPrimitiveArrayElement(env: *mut JNIEnv, arr: jobject, index: jint, v: jvalue, vCode: c_uchar) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_NewArray(env: *mut JNIEnv<'static>, eltClass: jclass, length: jint) -> jobject {
    let vm = unsafe{&*(*env).vm};
    let clazz = vm.resolve_class_object_by_jclass(eltClass);
    // FIXME check if we only have objects or primitives too (and if its always 1 dimensional)
    // one could use FieldType::from_str to fix, but then the prefilled values are wrong
    // maybe there is function somewhere which creates null values per fieldtype idk anymore
    let content = vec![vm.null(); length as usize];
    let arr = native_init_wrap!(env, vm.new_array(
        1,
        FieldType::Object(clazz.name.clone()).to_array_field_type(1),
        RefCell::new(content.clone())
    ));
    arr.id.nid() as jobject
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_NewMultiArray(env: *mut JNIEnv, eltClass: jclass, dim: jintArray) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCallerClass(env: *mut JNIEnv, n: i32) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindPrimitiveClass(env: *mut JNIEnv, utf: *const c_char) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ResolveClass(env: *mut JNIEnv, cls: jclass) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindClassFromClassLoader(env: *mut JNIEnv, name: *const c_char, init: jboolean, loader: jobject, throwError: jboolean) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindClassFromBootLoader(env: *mut JNIEnv, name: *const c_char) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindClassFromCaller(env: *mut JNIEnv, name: *const c_char, init: jboolean, loader: jobject, caller: jclass) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindClassFromClass(env: *mut JNIEnv, name: *const c_char, init: jboolean, from: jclass) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindLoadedClass(env: *mut JNIEnv, loader: jobject, name: jstring) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DefineClass(env: *mut JNIEnv, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize, pd: jobject) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DefineClassWithSource(env: *mut JNIEnv, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize, pd: jobject, source: *const c_char) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DefineClassWithSourceCond(env: *mut JNIEnv, name: *const c_char, loader: jobject, buf: *const jbyte, len: jsize, pd: jobject, source: *const c_char, verify: jboolean) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassName(env: *mut JNIEnv, cls: jclass) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassInterfaces(env: *mut JNIEnv, cls: jclass) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassLoader(env: *mut JNIEnv, cls: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsInterface(env: *mut JNIEnv, cls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassSigners(env: *mut JNIEnv, cls: jclass) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetClassSigners(env: *mut JNIEnv, cls: jclass, signers: jobjectArray) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetProtectionDomain(env: *mut JNIEnv, cls: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsArrayClass(env: *mut JNIEnv, cls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsPrimitiveClass(env: *mut JNIEnv, cls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetComponentType(env: *mut JNIEnv, cls: jclass) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassModifiers(env: *mut JNIEnv, cls: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetDeclaredClasses(env: *mut JNIEnv, ofClass: jclass) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetDeclaringClass(env: *mut JNIEnv, ofClass: jclass) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassSignature(env: *mut JNIEnv, cls: jclass) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassAnnotations(env: *mut JNIEnv, cls: jclass) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetFieldAnnotations(env: *mut JNIEnv, field: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodAnnotations(env: *mut JNIEnv, method: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodDefaultAnnotationValue(env: *mut JNIEnv, method: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodParameterAnnotations(env: *mut JNIEnv, method: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassTypeAnnotations(env: *mut JNIEnv, cls: jclass) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetFieldTypeAnnotations(env: *mut JNIEnv, field: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodTypeAnnotations(env: *mut JNIEnv, method: jobject) -> jbyteArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassDeclaredMethods(env: *mut JNIEnv, ofClass: jclass, publicOnly: jboolean) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassDeclaredFields(env: *mut JNIEnv, ofClass: jclass, publicOnly: jboolean) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassDeclaredConstructors(env: *mut JNIEnv, ofClass: jclass, publicOnly: jboolean) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassAccessFlags(env: *mut JNIEnv, cls: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassConstantPool(env: *mut JNIEnv, cls: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetSize(env: *mut JNIEnv, obj: jobject, unused: jobject) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetClassAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetClassAtIfLoaded(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jclass {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetMethodAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetMethodAtIfLoaded(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetFieldAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetFieldAtIfLoaded(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetMemberRefInfoAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetIntAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetLongAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetFloatAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jfloat {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetDoubleAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jdouble {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetStringAt(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ConstantPoolGetUTF8At(env: *mut JNIEnv, obj: jobject, unused: jobject, index: jint) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DoPrivileged(env: *mut JNIEnv, cls: jclass, action: jobject, context: jobject, wrapException: jboolean) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetInheritedAccessControlContext(env: *mut JNIEnv, cls: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetStackAccessControlContext(env: *mut JNIEnv, cls: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RegisterSignal(sig: jint, handler: *const c_void) -> *const c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RaiseSignal(sig: jint) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_FindSignal(name: *const c_char) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DesiredAssertionStatus(env: *mut JNIEnv, unused: jclass, cls: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_AssertionStatusDirectives(env: *mut JNIEnv, unused: jclass) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SupportsCX8() -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_CX8Field(env: *mut JNIEnv, obj: jobject, fldID: jfieldID, oldVal: jlong, newVal: jlong) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DTraceGetVersion(env: *mut JNIEnv) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DTraceActivate(env: *mut JNIEnv, version: jint, module_name: jstring, providers_count: jint, providers: *mut JVM_DTraceProvider) -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DTraceIsProbeEnabled(env: *mut JNIEnv, method: jmethodID) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DTraceDispose(env: *mut JNIEnv, handle: jlong) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_DTraceIsSupported(env: *mut JNIEnv) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassNameUTF(env: *mut JNIEnv, cb: jclass) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassCPTypes(env: *mut JNIEnv, cb: jclass, types: *const c_uchar) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassCPEntriesCount(env: *mut JNIEnv, cb: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassFieldsCount(env: *mut JNIEnv, cb: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetClassMethodsCount(env: *mut JNIEnv, cb: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxExceptionIndexes(env: *mut JNIEnv, cb: jclass, method_index: jint, exceptions: *const c_ushort) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxExceptionsCount(env: *mut JNIEnv, cb: jclass, method_index: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxByteCode(env: *mut JNIEnv, cb: jclass, method_index: jint, code: *const c_uchar) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxByteCodeLength(env: *mut JNIEnv, cb: jclass, method_index: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxExceptionTableEntry(env: *mut JNIEnv, cb: jclass, method_index: jint, entry_index: jint, entry: *mut JVM_ExceptionTableEntryType) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxExceptionTableLength(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetFieldIxModifiers(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxModifiers(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxLocalsCount(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxArgsSize(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxMaxStack(env: *mut JNIEnv, cb: jclass, index: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsConstructorIx(env: *mut JNIEnv, cb: jclass, index: i32) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsVMGeneratedMethodIx(env: *mut JNIEnv, cb: jclass, index: i32) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetMethodIxSignatureUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPFieldNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPMethodNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPMethodSignatureUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPFieldSignatureUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPClassNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPFieldClassNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPMethodClassNameUTF(env: *mut JNIEnv, cb: jclass, index: jint) -> *const c_char {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPFieldModifiers(env: *mut JNIEnv, cb: jclass, index: i32, calledClass: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetCPMethodModifiers(env: *mut JNIEnv, cb: jclass, index: i32, calledClass: jclass) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_ReleaseUTF(utf: *const c_char) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_IsSameClassPackage(env: *mut JNIEnv, class1: jclass, class2: jclass) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetLastErrorString(buf: *mut c_char, len: i32) -> jint {
    let last_error = std::io::Error::last_os_error().to_string();
    let error_string = CString::from_str(last_error.as_str()).unwrap();
    unsafe{
        *buf = *error_string.as_ptr()
    }
    (error_string.count_bytes() + 1) as jint
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_NativePath(path: *const c_char) -> *const c_char {
    path
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Open(fname: *const c_char, flags: jint, mode: jint) -> jint {
    let file_name = unsafe{CStr::from_ptr(fname)};
    let file_name = file_name.to_string_lossy();
    let path = Path::new(file_name.as_ref());
    if let Ok(file) = OpenOptions::new().read(true).open(path){
        let fd = file.into_raw_fd() as jint;
        fd
    } else {
        unimplemented!("NATIVE: JVM_Open: {}", file_name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Close(fd: jint) -> jint {
    let fd = unsafe{OwnedFd::from_raw_fd(RawFd::from_raw_fd(fd))};
    drop(fd);
    0 as jint //success
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Read(fd: jint, buf: *const c_char, nbytes: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Write(fd: jint, buf: *const c_char, nbytes: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Available(fd: jint, pbytes: *const jlong) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Lseek(fd: jint, offset: jlong, whence: jint) -> jlong {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetLength(fd: jint, length: jlong) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Sync(fd: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InitializeSocketLibrary() -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Socket(domain: jint, typ: jint, protocol: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SocketClose(fd: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SocketShutdown(fd: jint, howto: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Recv(fd: jint, buf: *const c_char, nBytes: jint, flags: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Send(fd: jint, buf: *const c_char, nBytes: jint, flags: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Timeout(fd: i32, timeout: c_long) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Listen(fd: jint, count: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Connect(fd: jint, him: *const sockaddr, len: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Bind(fd: jint, him: *const sockaddr, len: jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_Accept(fd: jint, him: *const sockaddr, len: *const jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RecvFrom(fd: jint, buf: *const c_char, nBytes: i32, flags: i32, from: *const sockaddr, fromlen: *const c_int) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SendTo(fd: jint, buf: *const c_char, len: i32, flags: i32, to: *const sockaddr, tolen: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SocketAvailable(fd: jint, result: *const jint) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetSockName(fd: jint, him: *const sockaddr, len: *const c_int) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetSockOpt(fd: jint, level: i32, optname: i32, optval: *const c_char, optlen: *const c_int) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_SetSockOpt(fd: jint, level: i32, optname: i32, optval: *const c_char, optlen: i32) -> jint {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetHostName(name: *const c_char, namelen: i32) -> i32 {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RawMonitorCreate() -> *const c_void {
    debug!(target: "native", "JVM_RawMonitorCreate");
    let data = Box::new("Monitor");
    Box::leak(data).as_ptr() as _
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RawMonitorDestroy(mon: *const c_void) -> c_void{
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RawMonitorEnter(mon: *const c_void) -> jint {
    debug!(target: "native", "JVM_RawMonitorEnter");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_RawMonitorExit(mon: *const c_void) {
    debug!(target: "native", "JVM_RawMonitorEnter");
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InvokeMethod(env: *mut JNIEnv, method: jobject, obj: jobject, args0: jobjectArray) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_NewInstanceFromConstructor(env: *mut JNIEnv, c: jobject, args0: jobjectArray) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetManagement(version: jint) -> *const c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_InitAgentProperties(env: *mut JNIEnv, agent_props: jobject) -> jobject {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetTemporaryDirectory(env: *mut JNIEnv) -> jstring {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetEnclosingMethodInfo(env: *mut JNIEnv, ofClass: jclass) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetThreadStateValues(env: *mut JNIEnv, javaThreadState: jint) -> jintArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetThreadStateNames(env: *mut JNIEnv, javaThreadState: jint, values: jintArray) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_KnownToNotExist(env: *mut JNIEnv, loader: jobject, classname: *const c_char) -> jboolean {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetResourceLookupCacheURLs(env: *mut JNIEnv, loader: jobject) -> jobjectArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetResourceLookupCache(env: *mut JNIEnv, loader: jobject, resource_name: *const c_char) -> jintArray {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn JVM_GetVersionInfo(env: *mut JNIEnv, info: *const jvm_version_info, info_size: isize) -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn jio_fprintf() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn jio_vsnprintf() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn jio_snprintf() -> c_void {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "system-unwind" fn jio_vfprintf() -> c_void {
    unimplemented!();
}