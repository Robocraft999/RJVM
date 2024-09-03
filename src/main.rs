use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::str::FromStr;
use log::{error, LevelFilter};
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

struct ClassFile{
    magic: u32,
    class_file_version: ClassFileVersion,
    constant_pool: ConstantPool,
    access_flags: ClassFlags,
    name: String,
    super_class: Option<String>,
    interfaces: Vec<String>,
    fields: Vec<FieldInfo>,
    methods: Vec<MethodInfo>,
    deprecated: bool,
    source_file: Option<String>
}

impl Debug for ClassFile{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassFile")
            .field("magic", &format_args!("{:x}", self.magic))
            .field("class_file_version", &self.class_file_version)
            .field("constant_pool", &format_args!("{:#?}", self.constant_pool))
            .field("access_flags", &format_args!("{:#?}", self.access_flags))
            .field("class_name", &self.name)
            .field("super_class", &self.super_class)
            .field("interfaces", &format_args!("{:#?}", self.interfaces))
            .field("fields", &format_args!("{:#?}", self.fields))
            .field("methods", &format_args!("{:#?}", self.methods))
            .field("deprecated", &self.deprecated)
            .field("source_file", &self.source_file)
            .finish()
    }
}

pub fn get_constant_printable(constant_pool: &ConstantPool, index: u16) -> String{
    let constant = constant_pool.0.get(index as usize - 1).expect(format!("Constant at index {} not found", index -1).as_str());
    match constant.clone(){
        ConstantPoolEntry::Utf8(string) => {
            string.to_string()
        }
        ConstantPoolEntry::Methodref(class_index, name_and_type_index) | ConstantPoolEntry::Fieldref(class_index, name_and_type_index) | ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index)=> {
            format!("{}.{}", get_constant_printable(constant_pool, class_index), get_constant_printable(constant_pool, name_and_type_index))
        }
        ConstantPoolEntry::NameAndType(name_index, descriptor_index) => {
            format!("{} {}", get_constant_printable(constant_pool, name_index), get_constant_printable(constant_pool, descriptor_index))
        }
        ConstantPoolEntry::Class(name_index) => {
            format!("{}", get_constant_printable(constant_pool, name_index))
        }
        ConstantPoolEntry::String(string_index) => {
            format!("{}", get_constant_printable(constant_pool, string_index))
        }
        ConstantPoolEntry::Integer(value) => {
            format!("{}", value)
        }
        ConstantPoolEntry::Long(value) => {
            format!("{}", value)
        }
        ConstantPoolEntry::Float(value) => {
            format!("{}", value)
        }
        ConstantPoolEntry::Double(value) => {
            format!("{}", value)
        }
        ConstantPoolEntry::Dummy => {String::new()}
        _ => unimplemented!("Constant with type {:?} is not printable", constant)
    }
}

fn parse_class_file(class_path: &ClassPath, class_name: &str) -> Result<ClassFile, ClassParseError> {
    //TODO error handling
    let mut bytes = class_path.resolve(class_name)?.expect("Could not load class").into_iter();

    let magic = parse_u4(&mut bytes)?;
    let _minor_version = parse_u2(&mut bytes)?;
    let major_version = parse_u2(&mut bytes)?;
    let class_file_version = ClassFileVersion::from_repr(major_version).expect(format!("Could not parse ClassFileVersion {}", major_version).as_str());
    let constant_pool_count: u32 = parse_u2(&mut bytes)?.into();
    let mut constant_pool_entries = Vec::new();
    let mut i = 0;
    let mut double_spaced = false;
    while i < constant_pool_count - 1{
        double_spaced = false;
        let tag = ConstantPoolEntry::from_repr(parse_u1(&mut bytes)?).expect("Unknown type of Constant");
        let constant_pool_entry = match tag {
            ConstantPoolEntry::Class(_) => {
                let name_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::Class(name_index)
            }
            ConstantPoolEntry::Fieldref(_, _) => {
                let class_index = parse_u2(&mut bytes)?;
                let name_and_type_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::Fieldref(class_index, name_and_type_index)
            }
            ConstantPoolEntry::Methodref(_, _) => {
                let class_index = parse_u2(&mut bytes)?;
                let name_and_type_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::Methodref(class_index, name_and_type_index)
            }
            ConstantPoolEntry::InterfaceMethodref(_, _) => {
                let class_index = parse_u2(&mut bytes)?;
                let name_and_type_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::InterfaceMethodref(class_index, name_and_type_index)
            }
            ConstantPoolEntry::NameAndType(_, _) => {
                let name_index = parse_u2(&mut bytes)?;
                let descriptor_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::NameAndType(name_index, descriptor_index)
            }
            ConstantPoolEntry::Utf8(_) => {
                let length = parse_u2(&mut bytes)?;
                let mut string_bytes = Vec::new();
                for _ in 0..length{
                    string_bytes.push(parse_u1(&mut bytes)?)
                }
                ConstantPoolEntry::Utf8(String::from_utf8(string_bytes).expect("String could not be parsed"))
            }
            ConstantPoolEntry::String(_) => {
                let string_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::String(string_index)
            }
            ConstantPoolEntry::Integer(_) => {
                let integer_bytes = parse_u4(&mut bytes)?;
                ConstantPoolEntry::Integer(integer_bytes as i32)
            }
            ConstantPoolEntry::Long(_) => {
                let bytes = parse_u8(&mut bytes)?;
                double_spaced = true;
                ConstantPoolEntry::Long(bytes as i64)
            }
            ConstantPoolEntry::Float(_) => {
                let bytes = parse_u4(&mut bytes)?;
                ConstantPoolEntry::Float(f32::from_bits(bytes))
            }
            ConstantPoolEntry::Double(_) => {
                let bytes = parse_u8(&mut bytes)?;
                //println!("DOUBLE {:?} {:?}, {:?}, {:?}, {:?}", bytes.to_be_bytes(), bytes as f64, 13.5f64.to_be_bytes(), parse_u8(&mut 13.5f64.to_be_bytes().to_vec().into_iter()).unwrap() as f64, f64::from_bits(bytes));
                double_spaced = true;
                ConstantPoolEntry::Double(f64::from_bits(bytes))
            }
            ConstantPoolEntry::InvokeDynamic(_, _) => {
                let bootstrap_method_attr_index = parse_u2(&mut bytes)?;
                let name_and_type_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::InvokeDynamic(bootstrap_method_attr_index, name_and_type_index)
            }
            ConstantPoolEntry::MethodHandle(_, _) => {
                let reference_kind = parse_u1(&mut bytes)?;
                let reference_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::MethodHandle(reference_kind, reference_index)
            }
            ConstantPoolEntry::MethodType(_) => {
                let descriptor_index = parse_u2(&mut bytes)?;
                ConstantPoolEntry::MethodType(descriptor_index)
            }
            _ => unimplemented!("CPTag {tag:?} not supported yet")
        };
        constant_pool_entries.push(constant_pool_entry);
        if double_spaced{
            i += 1;
            constant_pool_entries.push(ConstantPoolEntry::Dummy);
        }
        i += 1;
    }
    let constant_pool = ConstantPool(constant_pool_entries);
    let access_flags = parse_class_flags(parse_u2(&mut bytes)?);
    let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
    println!("Class name: {}", &name);
    let super_class_index = parse_u2(&mut bytes)?;
    let super_class = if super_class_index > 0{
        Some(get_constant_printable(&constant_pool, super_class_index))
    } else {
        None
    };

    let interfaces_count = parse_u2(&mut bytes)?;

    let mut interfaces = Vec::new();
    for _ in 0..interfaces_count{
        interfaces.push(get_constant_printable(&constant_pool, parse_u2(&mut bytes)?));
    }

    let fields_count = parse_u2(&mut bytes)?;
    let mut fields = Vec::new();
    for _ in 0..fields_count{
        let flags = parse_field_flags(parse_u2(&mut bytes)?);
        let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let descriptor = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let field_type = field_type_from_str(descriptor.as_str());
        let attributes_count = parse_u2(&mut bytes)?;
        let mut attributes = Vec::new();
        let mut deprecated = false;
        let mut constant_value = None;
        for _ in 0..attributes_count{
            let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
            let attribute_length = parse_u4(&mut bytes)?;

            match name.as_str() {
                "ConstantValue" => {
                    let constant_index = parse_u2(&mut bytes)?;
                    constant_value = Some(ConstantValue{
                        constant_index
                    })
                }
                _ => {
                    let mut info = Vec::new();
                    for _ in 0..attribute_length{
                        info.push(parse_u1(&mut bytes)?)
                    }

                    attributes.push(Attribute{
                        name,
                        info
                    });
                }
            }

            
        }
        fields.push(FieldInfo{
            flags,
            name,
            field_type,
            deprecated,
            constant_value,
            attributes
        });
    }

    let method_count = parse_u2(&mut bytes)?;
    let mut methods = Vec::new();
    for _ in 0..method_count{
        let flags = parse_method_flags(parse_u2(&mut bytes)?);
        let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let descriptor_str = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let descriptor = MethodDescriptor::new(descriptor_str);
        let attributes_count = parse_u2(&mut bytes)?;
        let mut attributes = Vec::new();
        let mut deprecated = false;
        let mut code = None;
        for _ in 0..attributes_count{
            let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
            let attribute_length = parse_u4(&mut bytes)?;
            let mut line_number_table = None;

            match name.as_str() {
                "Code" => {
                    let max_stack = parse_u2(&mut bytes)?;
                    let max_locals = parse_u2(&mut bytes)?;
                    let code_length = parse_u4(&mut bytes)?;
                    let mut code_bytes = Vec::new();
                    for _ in 0..code_length{
                        code_bytes.push(parse_u1(&mut bytes)?)
                    }

                    let exception_table_length = parse_u2(&mut bytes)?;
                    for _ in 0..exception_table_length{
                        let start_pc = parse_u2(&mut bytes)?;
                        let end_pc = parse_u2(&mut bytes)?;
                        let handler_pc = parse_u2(&mut bytes)?;
                        let catch_type = parse_u2(&mut bytes)?;
                    }
                    let code_attribute_count = parse_u2(&mut bytes)?;
                    let mut code_attributes = Vec::new();
                    for _ in 0..code_attribute_count{
                        let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
                        let attribute_length = parse_u4(&mut bytes)?;
                        match name.as_str() {
                            "LineNumberTable" => {
                                let line_number_table_length = parse_u2(&mut bytes)?;
                                let mut entries = Vec::new();
                                for _ in 0..line_number_table_length{
                                    let start_pc = parse_u2(&mut bytes)?;
                                    let line_number = parse_u2(&mut bytes)?;
                                    entries.push(LineNumberTableEntry::new(ProgramCounter(start_pc), LineNumber(line_number)))
                                }
                                line_number_table = Some(LineNumberTable(entries))
                            }
                            _ =>{
                                let mut info = Vec::new();
                                for _ in 0..attribute_length{
                                    info.push(parse_u1(&mut bytes)?)
                                }
                                code_attributes.push(Attribute{
                                    name,
                                    info
                                })
                            }
                        }
                    }

                    code = Some(Code{
                        max_stack,
                        max_locals,
                        code: code_bytes,
                        attributes: code_attributes,
                        line_number_table,
                    });
                }
                "Deprecated" => {
                    deprecated = true
                }
                _ => {
                    let mut info = Vec::new();
                    for _ in 0..attribute_length{
                        info.push(parse_u1(&mut bytes)?)
                    }

                    attributes.push(Attribute{
                        name,
                        info
                    });
                }
            }
        }
        methods.push(MethodInfo{
            flags,
            name,
            descriptor,
            deprecated,
            attributes,
            code,
        });
    }
    let attributes_count = parse_u2(&mut bytes)?;
    let mut deprecated = false;
    let mut source_file = None;

    for _ in 0..attributes_count{
        let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let attribute_length = parse_u4(&mut bytes)?;

        match name.as_str() {
            "SourceFile" => {
                source_file = Some(get_constant_printable(&constant_pool, parse_u2(&mut bytes)?))
            }
            "Deprecated" => {deprecated = true}
            _ => {
                let mut info = Vec::new();
                for _ in 0..attribute_length{
                    info.push(parse_u1(&mut bytes)?)
                }
            }
        }
    }

    let class_file = ClassFile{
        magic,
        class_file_version,
        constant_pool,
        access_flags,
        name,
        super_class,
        interfaces,
        fields,
        methods,
        deprecated,
        source_file
    };

    println!("------------------------------------");
    println!("{:#?}", class_file);

    println!("------------------------------------");
    for i in 0..class_file.constant_pool.0.len(){
        //println!("[{}] {} {:?}", i+1, get_constant_printable(&class_file.constant_pool, i as u16 + 1), &class_file.constant_pool.0.get(i).unwrap());
    }

    Ok(class_file)
}

fn main() {
    simple_logger::SimpleLogger::new().with_level(LevelFilter::Trace).without_timestamps().init().unwrap();

    let mut class_path = ClassPath::default();
    class_path.push("resources;resources/rt.jar").expect("TODO: panic message");

    let mut vm = VM::new(class_path);
    //vm.class_manager.get_or_resolve_class("Empty").expect("TODO: panic message");
    let main_method = vm.resolve_class_method("Main", "main", "([Ljava/lang/String;)I").unwrap();
    let result = vm.invoke(main_method, None, vec![Value::Null]);
    match result {
        Ok(res) => {
            println!("result: {res:?}");
            for (id, static_object) in vm.static_class_objects.iter(){
                println!("[{}] {:?}", vm.find_class_by_id(id.clone()).unwrap().name, static_object);
            }
        }
        Err(error) => {
            error!("Error: {}", error);
            vm.print_call_stack()
        }
    }
    //parse_class_file(&class_path, "java/lang/Exception");

    //parse_class_file(&class_path, "Main")?;
    //parse_class_file(&class_path, "java/lang/Object")?;
    //parse_class_file(&class_path, "java/io/PrintStream")?;
}