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
use crate::field_info::{field_type_from_str, FieldInfo};
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::vm::{VM, VmError};
use crate::vm::class::ClassAndMethod;
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

fn init_vm(vm: &mut VM) -> Result<(), VmError>{
    let vm_class = vm.get_or_resolve_class("sun/misc/VM")?;
    let properties_object = vm.new_object("java/util/Properties")?;
    let arg1 = vm.new_string_object("java.lang.Integer.IntegerCache.high".to_string())?;
    let arg2 = vm.new_string_object("127".to_string())?;
    let properties_init_method = vm.resolve_class_method("java/util/Properties", "<init>", "()V")?;
    vm.invoke_new_frame(properties_init_method, Some(properties_object), vec![])?;
    let propeties_set_method = vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    vm.invoke_new_frame(propeties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)])?;
    let save_properties_method = vm.resolve_class_method("sun/misc/VM", "saveAndRemoveProperties", "(Ljava/util/Properties;)V")?;
    vm.invoke_new_frame(save_properties_method, None, vec![Value::Reference(properties_object)])?;

    Ok(())
}

fn init_system(vm: &mut VM) -> Result<(), VmError>{
    let system_class = vm.get_or_resolve_class("java/lang/System")?;
    let static_object = vm.get_static_class_object(system_class.id).unwrap();

    let properties_object = vm.new_object("java/util/Properties")?;
    let properties_init = vm.resolve_class_method("java/util/Properties", "<init>", "()V")?;
    vm.invoke_new_frame(properties_init, Some(properties_object), vec![])?;

    let arg1 = vm.new_string_object("file.encoding".to_string())?;
    let arg2 = vm.new_string_object("UTF-8".to_string())?;
    let properties_set_method = vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    vm.invoke_new_frame(properties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)])?;

    let arg1 = vm.new_string_object("line.separator".to_string())?;
    let arg2 = vm.new_string_object("\n".to_string())?;
    let properties_set_method = vm.resolve_class_method("java/util/Properties", "setProperty", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;")?;
    vm.invoke_new_frame(properties_set_method, Some(properties_object), vec![Value::Reference(arg1), Value::Reference(arg2)])?;

    static_object.set_field(5, Value::Reference(properties_object));

    if let Some(setout0_method) = system_class.find_method("setOut0", "(Ljava/io/PrintStream;)V"){
        let class_and_method = ClassAndMethod{
            class: system_class,
            method: setout0_method,
        };
        let file_descriptor = vm.new_object("java/io/FileDescriptor").unwrap();
        let static_file_descriptor = vm.get_static_class_object(file_descriptor.class_id).unwrap();
        //public static final FileDescriptor out = new FileDescriptor(1);
        let file_descriptor_out = static_file_descriptor.get_field(3);
        let file_output_stream = vm.new_object("java/io/FileOutputStream")?;
        let file_output_stream_init = vm.resolve_class_method("java/io/FileOutputStream", "<init>", "(Ljava/io/FileDescriptor;)V")?;
        vm.invoke_new_frame(file_output_stream_init, Some(file_output_stream), vec![file_descriptor_out])?;

        let buffered_output_stream = vm.new_object("java/io/BufferedOutputStream")?;
        let buffered_output_stream_init = vm.resolve_class_method("java/io/BufferedOutputStream", "<init>", "(Ljava/io/OutputStream;I)V")?;
        vm.invoke_new_frame(buffered_output_stream_init, Some(buffered_output_stream), vec![Value::Reference(file_output_stream), Value::Integer(128)])?;

        let print_stream = vm.new_object("java/io/PrintStream")?;
        let print_stream_init = vm.resolve_class_method("java/io/PrintStream", "<init>", "(Ljava/io/OutputStream;Z)V")?;
        vm.invoke_new_frame(print_stream_init, Some(print_stream), vec![Value::Reference(buffered_output_stream), Value::Integer(1)])?;

        //vm.invoke(class_and_method, Some(static_object), vec![Value::Reference(print_stream)])?;
        static_object.set_field(1, Value::Reference(print_stream));
    }
    Ok(())
}

fn run_and_catch_method(vm: &mut VM, class_name: &str, method_name: &str, method_descriptor: &str){
    let main_method = vm.resolve_class_method(class_name, method_name, method_descriptor).unwrap();
    let result = vm.invoke_new_frame(main_method, None, vec![Value::Null]);
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
        }
    }
}

fn main() {
    let mut class_path = ClassPath::default();
    class_path.push("resources;resources/rt.jar;resources/LogicSim.jar;resources/lib/unix;resources/lib").expect("TODO: panic message");

    let mut vm = VM::new(class_path);
    simple_logger::SimpleLogger::new().with_level(LevelFilter::Warn).without_timestamps().init().unwrap();
    run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");
    info!("frames: {:?}", vm.call_stack.frames);
    return;

    match init_vm(&mut vm) {
        Ok(_) => {}
        Err(error) => {
            error!("Init VM: {}", error);
            //panic!();
        }
    };
    //vm.get_or_resolve_class("java/lang/CharacterData").expect("msg");
    match init_system(&mut vm){
        Ok(_) => {}
        Err(error) => {
            println!("'{}'", error);
            error!("Init System: {}", error);
            //panic!();
        }
    }

    simple_logger::SimpleLogger::new().with_level(LevelFilter::Error).without_timestamps().init().unwrap();
    println!("Init complete. Starting Main Program");

    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    //run_and_catch_method(&mut vm, "Test", "main", "([Ljava/lang/String;)V");
    run_and_catch_method(&mut vm, "logicsim/App", "main", "([Ljava/lang/String;)V");

    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}