#![feature(negative_impls)]
#![feature(c_variadic)]

use log::{error, LevelFilter};
use std::cell::RefCell;
use std::env;
use vm::class_path::ClassPath;

use crate::application::Application;
use crate::attribute::ProgramCounter;
use crate::field_info::FieldType;
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::jni::{self};
use crate::vm::value::Value;
use crate::vm::{VmError, VM};

mod application;
mod constants;
mod bytes;
mod access_flags;
mod attribute;
mod class_file_version;
mod field_info;
mod method_info;
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
    class_path.push("resources/rt.jar;resources/LogicSim.jar;resources/lib/unix;resources/lib").expect("TODO: panic message");

    println!("Booting up VM");
    let vm = Box::pin(VM::new(class_path));

    simple_logger::SimpleLogger::new().with_level(LevelFilter::Info).without_timestamps().init().unwrap();

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
    let app = Application::new(vm);
    app.startup();

    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    //run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");

    //simple_logger::SimpleLogger::new().with_level(LevelFilter::Warn).without_timestamps().init().unwrap();

    let args = env::args().skip(1).map(|s| Value::Reference(app.vm.try_new_string_object(&s).unwrap())).collect();
    let args_array = app.vm.try_new_array(1, FieldType::Object("java/lang/String".to_string()).to_array_field_type(1), RefCell::new(args)).unwrap();
    let p_args = vec![Value::Reference(args_array)];
    //run_and_catch_method(&mut vm, "de/klassenserver7b/k7bot/Main", "main", "([Ljava/lang/String;)V", p_args);
    //run_and_catch_method(&mut vm, "Hello", "main", "([Ljava/lang/String;)V", p_args);
    app.run_and_catch_method("logicsim/App", "main", "([Ljava/lang/String;)V", p_args);

    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}