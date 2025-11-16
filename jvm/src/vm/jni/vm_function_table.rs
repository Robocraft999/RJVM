use std::ffi::c_void;

use crate::vm::jni::types::JNIInvokeInterface_;

const METHOD_COUNT: usize = 3 + 5;
pub type JNIInvokeInterface = [*const c_void; METHOD_COUNT];
pub const METHODS: *const JNIInvokeInterface = &TABLE as _;
const TABLE: JNIInvokeInterface = [
    std::ptr::null() as _,
    std::ptr::null() as _,
    std::ptr::null() as _,
    JNIInvokeInterface_::DestroyJavaVM as _,
    JNIInvokeInterface_::AttachCurrentThread as _,
    JNIInvokeInterface_::DetachCurrentThread as _,
    JNIInvokeInterface_::GetEnv as _,
    JNIInvokeInterface_::AttachCurrentThreadAsDaemon as _,
];