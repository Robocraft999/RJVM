use std::ffi::c_void;

use crate::vm::jni::types::JNINativeInterface_;

const METHOD_COUNT: usize = 4 + 20;
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
];