use std::cell::RefCell;
use std::env;
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::str::FromStr;
use cesu8::from_java_cesu8;
use log::{error, info, warn, LevelFilter};
use vm::class_path::ClassPath;

use crate::application::Application;
use crate::attribute::{ProgramCounter};
use crate::field_info::{FieldInfo, FieldType};
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::jni::{self};
use crate::vm::{VmError, VM};
use crate::vm::value::Value;

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
                VMResultType::Ok(value) => value,
                VMResultType::NeedsClassInit(classes, reenter) => {return Ok(VMResultType::NeedsClassInit(classes, reenter))}
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
                Ok(VMResultType::Ok(value)) => value,
                Ok(VMResultType::NeedsClassInit(classes, reenter)) => {return Some(Ok(VMResultType::NeedsClassInit(classes, reenter)))}
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
                VMResultType::Ok(value) => ($wrapper)(value),
                VMResultType::NeedsClassInit(classes, reenter) => {return Ok(VMResultType::NeedsClassInit(classes, reenter))}
                _ => unreachable!("[get_after_init] got unexpected result {:?}", res)
            }
        }
    };
}

pub fn run() {
    let mut class_path = ClassPath::default();
    class_path.push("../resources/rt.jar;../resources/LogicSim.jar;../resources/lib/unix;../resources/lib").expect("TODO: panic message");

    println!("Booting up VM");
    let vm = VM::new(class_path);

    simple_logger::SimpleLogger::new().with_level(LevelFilter::Error).without_timestamps().init().unwrap();
    let env = JNIEnv{
        methods: jni::env_function_table::METHODS,
        vm: &vm,
    };
    let javavm = JavaVM{
        methods: jni::vm_function_table::METHODS,
        env
    };
    unsafe {
        use libffi::middle::{Closure, Cif, Type, Arg};
        use std::{ffi::c_void, ptr};
        let lib = libloading::Library::new("/home/admin/.jdks/temurin-1.8.0_462/jre/lib/amd64/libjava.so").unwrap();
        let sym: libloading::Symbol<*const ()> = lib.get(b"JNI_OnLoad").unwrap();

        let func_ptr = *sym as * const c_void;


        let vm_ptr = ptr::from_ref(&javavm) as *const c_void;
        let reserved = std::ptr::null() as *const c_void;
        let cif = Cif::new(vec![Type::pointer(), Type::pointer()], Type::i32()); //JNI_OnLoad
        let res: i32 = cif.call(libffi::low::CodePtr::from_ptr(func_ptr), &[Arg::new(&vm_ptr), Arg::new(&reserved)]);
        
    }
    let mut app = Application::new(javavm, vm);
    app.startup();
    todo!("Init complete");

    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    //run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");

    let args = env::args().skip(1).map(|s| Value::Reference(vm.try_new_string_object(s).unwrap())).collect();
    let args_array = vm.try_new_array(1, FieldType::Object("java/lang/String".to_string()).to_array_field_type(1), RefCell::new(args)).unwrap();
    let p_args = vec![Value::Reference(args_array)];
    //run_and_catch_method(&mut vm, "de/klassenserver7b/k7bot/Main", "main", "([Ljava/lang/String;)V", p_args);
    //run_and_catch_method(&mut vm, "Hello", "main", "([Ljava/lang/String;)V", p_args);
    app.run_and_catch_method("logicsim/App", "main", "([Ljava/lang/String;)V", p_args);

    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}