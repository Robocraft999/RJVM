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
    JNINativeInterface_::CallBooleanMethod as _,
    JNINativeInterface_::CallBooleanMethodV as _,
    JNINativeInterface_::CallBooleanMethodA as _,
    JNINativeInterface_::CallByteMethod as _,
    JNINativeInterface_::CallByteMethodV as _,
    JNINativeInterface_::CallByteMethodA as _,
    JNINativeInterface_::CallCharMethod as _,
    JNINativeInterface_::CallCharMethodV as _,
    JNINativeInterface_::CallCharMethodA as _,
    JNINativeInterface_::CallShortMethod as _,
    JNINativeInterface_::CallShortMethodV as _,
    JNINativeInterface_::CallShortMethodA as _,
    JNINativeInterface_::CallIntMethod as _,
    JNINativeInterface_::CallIntMethodV as _,
    JNINativeInterface_::CallIntMethodA as _,
    JNINativeInterface_::CallLongMethod as _,
    JNINativeInterface_::CallLongMethodV as _,
    JNINativeInterface_::CallLongMethodA as _,
    JNINativeInterface_::CallFloatMethod as _,
    JNINativeInterface_::CallFloatMethodV as _,
    JNINativeInterface_::CallFloatMethodA as _,
    JNINativeInterface_::CallDoubleMethod as _,
    JNINativeInterface_::CallDoubleMethodV as _,
    JNINativeInterface_::CallDoubleMethodA as _,
    JNINativeInterface_::CallVoidMethod as _,
    JNINativeInterface_::CallVoidMethodV as _,
    JNINativeInterface_::CallVoidMethodA as _,

    //64
    JNINativeInterface_::CallNonvirtualObjectMethod as _,
    JNINativeInterface_::CallNonvirtualObjectMethodV as _,
    JNINativeInterface_::CallNonvirtualObjectMethodA as _,
    JNINativeInterface_::CallNonvirtualBooleanMethod as _,
    JNINativeInterface_::CallNonvirtualBooleanMethodV as _,
    JNINativeInterface_::CallNonvirtualBooleanMethodA as _,
    JNINativeInterface_::CallNonvirtualByteMethod as _,
    JNINativeInterface_::CallNonvirtualByteMethodV as _,
    JNINativeInterface_::CallNonvirtualByteMethodA as _,
    JNINativeInterface_::CallNonvirtualCharMethod as _,
    JNINativeInterface_::CallNonvirtualCharMethodV as _,
    JNINativeInterface_::CallNonvirtualCharMethodA as _,
    JNINativeInterface_::CallNonvirtualShortMethod as _,
    JNINativeInterface_::CallNonvirtualShortMethodV as _,
    JNINativeInterface_::CallNonvirtualShortMethodA as _,
    JNINativeInterface_::CallNonvirtualIntMethod as _,
    JNINativeInterface_::CallNonvirtualIntMethodV as _,
    JNINativeInterface_::CallNonvirtualIntMethodA as _,
    JNINativeInterface_::CallNonvirtualLongMethod as _,
    JNINativeInterface_::CallNonvirtualLongMethodV as _,
    JNINativeInterface_::CallNonvirtualLongMethodA as _,
    JNINativeInterface_::CallNonvirtualFloatMethod as _,
    JNINativeInterface_::CallNonvirtualFloatMethodV as _,
    JNINativeInterface_::CallNonvirtualFloatMethodA as _,
    JNINativeInterface_::CallNonvirtualDoubleMethod as _,
    JNINativeInterface_::CallNonvirtualDoubleMethodV as _,
    JNINativeInterface_::CallNonvirtualDoubleMethodA as _,
    JNINativeInterface_::CallNonvirtualVoidMethod as _,
    JNINativeInterface_::CallNonvirtualVoidMethodV as _,
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
    JNINativeInterface_::CallStaticByteMethod as _,
    JNINativeInterface_::CallStaticByteMethodV as _,
    JNINativeInterface_::CallStaticByteMethodA as _,
    JNINativeInterface_::CallStaticCharMethod as _,
    JNINativeInterface_::CallStaticCharMethodV as _,
    JNINativeInterface_::CallStaticCharMethodA as _,
    JNINativeInterface_::CallStaticShortMethod as _,
    JNINativeInterface_::CallStaticShortMethodV as _,
    JNINativeInterface_::CallStaticShortMethodA as _,
    JNINativeInterface_::CallStaticIntMethod as _,
    JNINativeInterface_::CallStaticIntMethodV as _,
    JNINativeInterface_::CallStaticIntMethodA as _,
    JNINativeInterface_::CallStaticLongMethod as _,
    JNINativeInterface_::CallStaticLongMethodV as _,
    JNINativeInterface_::CallStaticLongMethodA as _,
    JNINativeInterface_::CallStaticFloatMethod as _,
    JNINativeInterface_::CallStaticFloatMethodV as _,
    JNINativeInterface_::CallStaticFloatMethodA as _,
    JNINativeInterface_::CallStaticDoubleMethod as _,
    JNINativeInterface_::CallStaticDoubleMethodV as _,
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
    JNINativeInterface_::NewBooleanArray as _,
    JNINativeInterface_::NewByteArray as _,
    JNINativeInterface_::NewCharArray as _,
    JNINativeInterface_::NewShortArray as _,
    JNINativeInterface_::NewIntArray as _,
    JNINativeInterface_::NewLongArray as _,
    JNINativeInterface_::NewFloatArray as _,
    JNINativeInterface_::NewDoubleArray as _,

    //183
    //GetPrimitiveArrayElements
    JNINativeInterface_::GetBooleanArrayElements as _,
    JNINativeInterface_::GetByteArrayElements as _,
    JNINativeInterface_::GetCharArrayElements as _,
    JNINativeInterface_::GetShortArrayElements as _,
    JNINativeInterface_::GetIntArrayElements as _,
    JNINativeInterface_::GetLongArrayElements as _,
    JNINativeInterface_::GetFloatArrayElements as _,
    JNINativeInterface_::GetDoubleArrayElements as _,

    //191
    //ReleasePrimitiveArrayElements
    JNINativeInterface_::ReleaseBooleanArrayElements as _,
    JNINativeInterface_::ReleaseByteArrayElements as _,
    JNINativeInterface_::ReleaseCharArrayElements as _,
    JNINativeInterface_::ReleaseShortArrayElements as _,
    JNINativeInterface_::ReleaseIntArrayElements as _,
    JNINativeInterface_::ReleaseLongArrayElements as _,
    JNINativeInterface_::ReleaseFloatArrayElements as _,
    JNINativeInterface_::ReleaseDoubleArrayElements as _,

    //199
    //GetPrimitiveArrayRegion
    JNINativeInterface_::GetBooleanArrayRegion as _,
    JNINativeInterface_::GetByteArrayRegion as _,
    JNINativeInterface_::GetCharArrayRegion as _,
    JNINativeInterface_::GetShortArrayRegion as _,
    JNINativeInterface_::GetIntArrayRegion as _,
    JNINativeInterface_::GetLongArrayRegion as _,
    JNINativeInterface_::GetFloatArrayRegion as _,
    JNINativeInterface_::GetDoubleArrayRegion as _,

    //207
    //SetPrimitiveArrayRegion
    JNINativeInterface_::SetBooleanArrayRegion as _,
    JNINativeInterface_::SetByteArrayRegion as _,
    JNINativeInterface_::SetCharArrayRegion as _,
    JNINativeInterface_::SetShortArrayRegion as _,
    JNINativeInterface_::SetIntArrayRegion as _,
    JNINativeInterface_::SetLongArrayRegion as _,
    JNINativeInterface_::SetFloatArrayRegion as _,
    JNINativeInterface_::SetDoubleArrayRegion as _,

    //215
    //reg / unreg natives
    JNINativeInterface_::RegisterNatives as _,
    JNINativeInterface_::UnregisterNatives as _,

    //217
    //monitor
    JNINativeInterface_::MonitorEnter as _,
    JNINativeInterface_::MonitorExit as _,

    //219
    //GetJavaVM
    JNINativeInterface_::GetJavaVM as _,

    //220
    //get string region
    JNINativeInterface_::GetStringRegion as _,
    JNINativeInterface_::GetStringUTFRegion as _,

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
    JNINativeInterface_::NewWeakGlobalRef as _,
    JNINativeInterface_::DeleteWeakGlobalRef as _,

    //228
    //exception check
    JNINativeInterface_::ExceptionCheck as _,

    //229
    //direct byte buffer
    JNINativeInterface_::NewDirectByteBuffer as _,
    JNINativeInterface_::GetDirectBufferAddress as _,
    JNINativeInterface_::GetDirectBufferCapacity as _,

    //232
    //get object ref type
    JNINativeInterface_::GetObjectRefType as _,
];