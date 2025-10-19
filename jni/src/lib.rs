pub mod types;
pub mod jvm;

use libffi::middle::{Closure, Cif, Type, Arg};
use std::{ffi::c_void, ptr};
use crate::types::JNIInvokeInterface_;

fn get_cif_from_sig(sig: &str) -> Cif{
    Cif::new(vec![Type::pointer(), Type::pointer()], Type::i32())
}

struct JNIInvokeInterface{
    methods: *const JNIInvokeInterface_
}

pub unsafe fn boostrap(){
    //let libjvm = libloading::Library::new("/usr/lib/jvm/java-21-openjdk/lib/server/libjvm.so").unwrap();
    let lib = libloading::Library::new("/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/libjava.so").unwrap();
    let sym: libloading::Symbol<*const ()> = lib.get(b"JNI_OnLoad").unwrap();

    let func_ptr = *sym as * const c_void;

    let cif = get_cif_from_sig("JNI_OnLoad");
    let mut d = types::JNIInvokeInterface_{
        reserved0: std::ptr::null() as *const c_void,
        reserved1: std::ptr::null() as *const c_void,
        reserved2: std::ptr::null() as *const c_void,
        a: JNIInvokeInterface_::DestroyJavaVM as _,                
        b: JNIInvokeInterface_::AttachCurrentThread as _,                
        c: JNIInvokeInterface_::DetachCurrentThread as _,                
        d: JNIInvokeInterface_::GetEnv as _,                
        e: JNIInvokeInterface_::AttachCurrentThreadAsDaemon as _,                

    };
    let vm = JNIInvokeInterface{
        methods: &d as _
    };
    let vm_ptr = ptr::from_ref(&vm) as *const c_void;
    let reserved = std::ptr::null() as *const c_void;
    let res: i32 = cif.call(libffi::low::CodePtr::from_ptr(func_ptr), &[Arg::new(&vm_ptr), Arg::new(&reserved)]);
    assert_eq!(res, types::JNI_VERSION_1_2);

}