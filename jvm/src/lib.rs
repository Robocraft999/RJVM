#![feature(negative_impls)]
#![feature(c_variadic)]
#![feature(iterator_try_collect)]
extern crate core;

use log::LevelFilter;
use parking_lot::RwLock;
use std::env;
use vm::class_path::ClassPath;

use crate::class_file::fields::field_type::FieldType;
use crate::vm::value::Value;
use crate::vm::VM;
use vm::application::Application;
use crate::vm::constants::classes::JAVA_LANG_STRING;

mod access_flags;
mod vm;
mod error;
mod bytecode;
mod class_file;

#[macro_export]
macro_rules! get_or_init {
    ($x:expr) => {
        {
            let res = $x;
            match res{
                VMResultType::Successful(value) => value,
                VMResultType::Interrupted(amount, reset_pc) => {return Ok(VMResultType::Interrupted(amount, reset_pc))}
                _ => unreachable!("[get_after_init] got unexpected result {:?}", res)
            }
        }
    };
}

#[macro_export]
macro_rules! get_or_init_option {
    ($x:expr) => {
        {
            let res = $x;
            match res{
                Ok(VMResultType::Successful(value)) => value,
                Ok(VMResultType::Interrupted(amount, reset_pc)) => {return Some(Ok(VMResultType::Interrupted(amount, reset_pc)))}
                Err(e) => {return Some(Err(e))}
                Ok(_) => unreachable!("[get_after_init] got unexpected result {:?}", res)
            }
        }
    };
}

#[macro_export]
macro_rules! get_or_init_special {
    ($x:expr, $wrapper:expr) => {
        {
            let res = $x;
            match res{
                VMResultType::Successful(value) => ($wrapper)(value),
                VMResultType::Interrupted(amount, reset_pc) => {return Ok(VMResultType::Interrupted(amount, reset_pc))}
                _ => unreachable!("[get_after_init] got unexpected result {:?}", res)
            }
        }
    };
}

pub fn run() {
    let mut class_path = ClassPath::default();
    class_path.push("resources/rt.jar;resources/LogicSim.jar;resources/lib/unix;resources/lib;resources/test").expect("TODO: panic message");

    println!("Booting up VM");

    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("debug", LevelFilter::Debug)
        .with_module_level("native", LevelFilter::Info)
        .without_timestamps()
        .with_threads(true)
        .init()
        .unwrap();

    /*unsafe {
        use libffi::middle::{Closure, Cif, Type, Arg};
        use std::{ffi::c_void, ptr};
        let lib = libloading::Library::new("/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/libjava.so").unwrap();
        let sym: libloading::Symbol<*const ()> = lib.get(b"JNI_OnLoad").unwrap();

        let func_ptr = *sym as * const c_void;


        let vm_ptr = ptr::from_ref(&javavm) as *const c_void;
        let reserved = std::ptr::null() as *const c_void;
        let cif = Cif::new(vec![Type::pointer(), Type::pointer()], Type::i32()); //JNI_OnLoad
        let res: i32 = cif.call(libffi::low::CodePtr::from_ptr(func_ptr), &[Arg::new(&vm_ptr), Arg::new(&reserved)]);
        
    }*/
    let mut app = Application::new(class_path);
    app.startup();

    /*simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("debug", LevelFilter::Debug)
        .with_module_level("native", LevelFilter::Debug)
        .without_timestamps()
        .init()
        .unwrap();*/



    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    //run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");

    app.start_user_code();

    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}