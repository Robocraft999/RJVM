use std::ffi::c_void;

use crate::vm::jni::types::JNINativeInterface_;

const METHOD_COUNT: usize = 4 + 229;
pub type JNINativeInterface = [*const c_void; METHOD_COUNT];
pub const METHODS: *const JNINativeInterface = &TABLE as _;
const TABLE: JNINativeInterface = [
    std::ptr::null() as _,
    std::ptr::null() as _,
    std::ptr::null() as _,
    std::ptr::null() as _,

    //4
    JNINativeInterface_::GetVersion as _,

    JNINativeInterface_::DefineClass as _,
    JNINativeInterface_::FindClass as _,

    JNINativeInterface_::FromReflectedMethod as _,
    JNINativeInterface_::FromReflectedField as _,

    JNINativeInterface_::ToReflectedMethod as _,

    //10
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
    //20
    JNINativeInterface_::PopLocalFrame as _,

    JNINativeInterface_::NewGlobalRef as _,
    JNINativeInterface_::DeleteGlobalRef as _,
    JNINativeInterface_::DeleteLocalRef as _,
    JNINativeInterface_::IsSameObject as _,
    JNINativeInterface_::NewLocalRef as _,
    JNINativeInterface_::EnsureLocalCapacity as _,

    JNINativeInterface_::AllocObject as _,
    //28
    JNINativeInterface_::NewObject as _,
    JNINativeInterface_::NewObjectV as _,
    JNINativeInterface_::NewObjectA as _,

    JNINativeInterface_::GetObjectClass as _,
    JNINativeInterface_::IsInstanceOf as _,

    JNINativeInterface_::GetMethodID as _,

    //34
    JNINativeInterface_::CallObjectMethod as _,
    JNINativeInterface_::CallObjectMethodV as _,
    JNINativeInterface_::CallObjectMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallBooleanMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallByteMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallCharMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallShortMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallIntMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallLongMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallFloatMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallDoubleMethodA as _,
    JNINativeInterface_::CallVoidMethod as _,
    JNINativeInterface_::CallVoidMethodV as _,
    JNINativeInterface_::CallVoidMethodA as _,

    //64
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualObjectMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualBooleanMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualByteMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualCharMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualShortMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualIntMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualLongMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualFloatMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualDoubleMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallNonvirtualVoidMethodA as _,

    //94
    JNINativeInterface_::GetFieldID as _,

    //95
    JNINativeInterface_::GetObjectField as _,
    JNINativeInterface_::GetBooleanField as _,
    JNINativeInterface_::GetByteField as _,
    JNINativeInterface_::GetCharField as _,
    JNINativeInterface_::GetShortField as _,
    JNINativeInterface_::GetIntField as _,
    JNINativeInterface_::GetLongField as _,
    JNINativeInterface_::GetFloatField as _,
    JNINativeInterface_::GetDoubleField as _,

    //104
    JNINativeInterface_::SetObjectField as _,
    JNINativeInterface_::SetBooleanField as _,
    JNINativeInterface_::SetByteField as _,
    JNINativeInterface_::SetCharField as _,
    JNINativeInterface_::SetShortField as _,
    JNINativeInterface_::SetIntField as _,
    JNINativeInterface_::SetLongField as _,
    JNINativeInterface_::SetFloatField as _,
    JNINativeInterface_::SetDoubleField as _,

    //113
    JNINativeInterface_::GetStaticMethodID as _,

    //114
    JNINativeInterface_::CallStaticObjectMethod as _,
    JNINativeInterface_::CallStaticObjectMethodV as _,
    JNINativeInterface_::CallStaticObjectMethodA as _,
    JNINativeInterface_::CallStaticBooleanMethod as _,
    JNINativeInterface_::CallStaticBooleanMethodV as _,
    JNINativeInterface_::CallStaticBooleanMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticByteMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticCharMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticShortMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticIntMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticLongMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticFloatMethodA as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::CallStaticDoubleMethodA as _,
    JNINativeInterface_::CallStaticVoidMethod as _,
    JNINativeInterface_::CallStaticVoidMethodV as _,
    JNINativeInterface_::CallStaticVoidMethodA as _,

    //144
    JNINativeInterface_::GetStaticFieldID as _,

    //145
    JNINativeInterface_::GetStaticObjectField as _,
    JNINativeInterface_::GetStaticBooleanField as _,
    JNINativeInterface_::GetStaticByteField as _,
    JNINativeInterface_::GetStaticCharField as _,
    JNINativeInterface_::GetStaticShortField as _,
    JNINativeInterface_::GetStaticIntField as _,
    JNINativeInterface_::GetStaticLongField as _,
    JNINativeInterface_::GetStaticFloatField as _,
    JNINativeInterface_::GetStaticDoubleField as _,

    //154
    JNINativeInterface_::SetStaticObjectField as _,
    JNINativeInterface_::SetStaticBooleanField as _,
    JNINativeInterface_::SetStaticByteField as _,
    JNINativeInterface_::SetStaticCharField as _,
    JNINativeInterface_::SetStaticShortField as _,
    JNINativeInterface_::SetStaticIntField as _,
    JNINativeInterface_::SetStaticLongField as _,
    JNINativeInterface_::SetStaticFloatField as _,
    JNINativeInterface_::SetStaticDoubleField as _,

    //163
    JNINativeInterface_::NewString as _,
    JNINativeInterface_::GetStringLength as _,
    JNINativeInterface_::GetStringChars as _,
    JNINativeInterface_::ReleaseStringChars as _,

    //167
    JNINativeInterface_::NewStringUTF as _,
    JNINativeInterface_::GetStringUTFLength as _,
    JNINativeInterface_::GetStringUTFChars as _,
    JNINativeInterface_::ReleaseStringUTFChars as _,

    //171
    JNINativeInterface_::GetArrayLength as _,

    //172
    //object array stuff
    JNINativeInterface_::NewObjectArray as _,
    JNINativeInterface_::GetObjectArrayElement as _,
    JNINativeInterface_::SetObjectArrayElement as _,

    //175
    //NewPrimitiveArray
    not_implemented as _,
    JNINativeInterface_::NewByteArray as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,

    //183
    //GetPrimitiveArrayElements
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,

    //191
    //ReleasePrimitiveArrayElements
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,

    //199
    //GetPrimitiveArrayRegion
    not_implemented as _,
    JNINativeInterface_::GetByteArrayRegion as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    JNINativeInterface_::GetLongArrayRegion as _,
    not_implemented as _,
    not_implemented as _,

    //207
    //SetPrimitiveArrayRegion
    not_implemented as _,
    JNINativeInterface_::SetByteArrayRegion as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,
    not_implemented as _,

    //215
    //reg / unreg natives
    not_implemented as _,
    not_implemented as _,

    //217
    //monitor
    not_implemented as _,
    not_implemented as _,

    //219
    //GetJavaVM
    JNINativeInterface_::GetJavaVM as _,

    //220
    //get string region
    not_implemented as _,
    not_implemented as _,

    //222
    //primitive array critical
    JNINativeInterface_::GetPrimitiveArrayCritical as _,
    JNINativeInterface_::ReleasePrimitiveArrayCritical as _,

    //224
    //string critical
    JNINativeInterface_::GetStringChars as _,
    JNINativeInterface_::ReleaseStringChars as _,

    //226
    //week global ref
    not_implemented as _,
    not_implemented as _,

    //228
    //exception check
    JNINativeInterface_::ExceptionCheck as _,

    //229
    //direct byte buffer
    JNINativeInterface_::NewDirectByteBuffer as _,
    not_implemented as _,
    not_implemented as _,

    //232
    //get object ref type
    not_implemented as _,
];

unsafe fn not_implemented() {
    unimplemented!();
}

unsafe fn test() {
    unimplemented!("tester")
}