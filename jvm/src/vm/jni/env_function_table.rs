use std::ffi::c_void;

use crate::vm::jni::types::JNINativeInterface_;

const METHOD_COUNT: usize = 4 + 90;
pub type JNINativeInterface = [*const c_void; METHOD_COUNT];
pub const METHODS: *const JNINativeInterface = &TABLE as _;
const TABLE: JNINativeInterface = [
    std::ptr::null() as _,
    std::ptr::null() as _,
    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::GetVersion as _,

    JNINativeInterface_::DefineClass as _,
    JNINativeInterface_::FindClass as _,

    JNINativeInterface_::FromReflectedMethod as _,
    JNINativeInterface_::FromReflectedField as _,

    JNINativeInterface_::ToReflectedMethod as _,

    JNINativeInterface_::GetSuperclass as _,
    JNINativeInterface_::IsAssignableFrom as _,

    JNINativeInterface_::ToReflectedField as _,

    JNINativeInterface_::Throw as _,
    JNINativeInterface_::ThrowNew as _,
    JNINativeInterface_::ExceptionOccurred as _,
    JNINativeInterface_::ExceptionDescribe as _,
    JNINativeInterface_::ExceptionClear as _,
    JNINativeInterface_::FatalError as _,

    JNINativeInterface_::PushLocalFrame as _,
    JNINativeInterface_::PopLocalFrame as _,

    JNINativeInterface_::NewGlobalRef as _,
    JNINativeInterface_::DeleteGlobalRef as _,
    JNINativeInterface_::DeleteLocalRef as _,
    JNINativeInterface_::IsSameObject as _,
    JNINativeInterface_::NewLocalRef as _,
    JNINativeInterface_::EnsureLocalCapacity as _,

    JNINativeInterface_::AllocObject as _,
    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::NewObjectA as _,

    JNINativeInterface_::GetObjectClass as _,
    JNINativeInterface_::IsInstanceOf as _,

    JNINativeInterface_::GetMethodID as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallObjectMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallBooleanMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallByteMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallCharMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallShortMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallIntMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallLongMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallFloatMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallDoubleMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallVoidMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualObjectMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualBooleanMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualByteMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualCharMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualShortMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualIntMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualLongMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualFloatMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualDoubleMethodA as _,

    std::ptr::null() as _,
    std::ptr::null() as _,
    JNINativeInterface_::CallNonvirtualVoidMethodA as _,
];