use std::cell::RefCell;
use std::env;
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::str::FromStr;
use cesu8::from_java_cesu8;
use log::{error, info, warn, LevelFilter};
use access_flags::{parse_class_flags, parse_field_flags, parse_method_flags};
use attribute::{Attribute, Code};
use vm::class_path::ClassPath;

use crate::access_flags::ClassFlags;
use crate::attribute::{ConstantValue, LineNumber, LineNumberTable, LineNumberTableEntry, ProgramCounter};
use crate::bytes::{parse_u1, parse_u2, parse_u4, parse_u8};
use crate::class_file_version::ClassFileVersion;
use crate::constants::*;
use crate::error::ClassParseError;
use crate::field_info::{FieldInfo, FieldType};
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::vm::{VM, VmError};
use crate::vm::class::{ClassAndMethod, ClassRef};
use crate::vm::result::VMResultType;
use crate::vm::value::Value;

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

fn init_vm(vm: &mut VM) -> Result<(), VmError>{
    if let VMResultType::NeedsClassInit(classes, _) = vm.get_or_resolve_class("sun/misc/VM")?{
        for frame in classes{
            vm.invoke_frame(frame)?;
        }
    }

    Ok(())
}

fn init_system(vm: &mut VM) -> Result<(), VmError>{
    if let VMResultType::NeedsClassInit(classes, _) = vm.get_or_resolve_class("java/lang/System")?{
        for frame in classes{
            vm.invoke_frame(frame)?;
        }
    }
    let init = vm.try_resolve_class_method("java/lang/System", "initializeSystemClass", "()V")?;
    vm.invoke_new_frame(init, None, vec![])?;
    Ok(())
}

fn run_and_catch_method<'a>(vm: &'a mut VM<'a>, class_name: &str, method_name: &str, method_descriptor: &str, args: Vec<Value<'a>>){
    if let VMResultType::NeedsClassInit(classes, _) = vm.get_or_resolve_class(class_name).unwrap().clone(){
        for frame in classes{
            vm.invoke_frame(frame).unwrap();
        }
    }
    let main_method = vm.try_resolve_class_method(class_name, method_name, method_descriptor).unwrap();
    let result = vm.invoke_new_frame(main_method, None, args);
    match result {
        Ok(res) => {
            println!("result: {res:?}");
            for (id, static_object) in vm.static_class_objects.iter(){
                //println!("[{}] {:?}", vm.find_class_by_id(id.clone()).unwrap().name, static_object);
            }
        }
        Err(error) => {
            error!("Error: {}", error);
            //vm.call_stack.print_call_stack();
            println!("Frames:");
            for (index, call_frame_info) in vm.call_stack.frames.iter().enumerate(){
                //error!("[{}]: {:?}, stack={}, locals={}", index, call_frame.pc, call_frame.stack, call_frame.locals);
                println!("[{}]: {:?}", index, call_frame_info);
            }
        }
    }
}

fn main() {
    let mut class_path = ClassPath::default();
    class_path.push("resources;resources/rt.jar;resources/LogicSim.jar;resources/k7bot-v1.24.0.jar;resources/lib/unix;resources/lib").expect("TODO: panic message");

    let mut vm = VM::new(class_path);
    
    /*simple_logger::SimpleLogger::new().with_level(LevelFilter::Warn).without_timestamps().init().unwrap();
    run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");
    return;*/

    match init_vm(&mut vm) {
        Ok(_) => {}
        Err(error) => {
            error!("Init VM: {}", error);
            panic!();
        }
    };
    //simple_logger::SimpleLogger::new().with_level(LevelFilter::Trace).without_timestamps().init().unwrap();
    //vm.get_or_resolve_class("java/lang/CharacterData").expect("msg");
    match init_system(&mut vm){
        Ok(_) => {}
        Err(error) => {
            println!("'{}'", error);
            error!("Init System: {}", error);
            panic!();
        }
    }

    simple_logger::SimpleLogger::new().with_level(LevelFilter::Warn).without_timestamps().init().unwrap();
    println!("Init complete. Starting Main Program");

    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    //run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");
    //run_and_catch_method(&mut vm, "logicsim/App", "main", "([Ljava/lang/String;)V");
    let args = env::args().skip(1).map(|s| Value::Reference(vm.try_new_string_object(s).unwrap())).collect();
    let args_array = vm.try_new_array(1, FieldType::Object("java/lang/String".to_string()).to_array_field_type(1), RefCell::new(args)).unwrap();
    let p_args = vec![Value::Reference(args_array)];
    run_and_catch_method(&mut vm, "de/klassenserver7b/k7bot/Main", "main", "([Ljava/lang/String;)V", p_args);

    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}