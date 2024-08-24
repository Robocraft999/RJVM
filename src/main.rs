mod constants;
mod bytes;

use std::fmt::{format, write, Debug, Formatter};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use bytes::{read_u2, read_u4};

use crate::bytes::{ByteType, parse_u1, parse_u2, parse_u4};
use crate::bytes::ByteType::{U1, U2, U4};
use crate::constants::*;

#[derive(Debug)]
struct CPInfo{
    tag: Constant,
    info: Vec<ByteType>
}

#[derive(Debug)]
struct MemberInfo{
    access_flags: ByteType,
    name_index: ByteType,
    descriptor_index: ByteType,
    attributes: Vec<AttributeInfo>
}

#[derive(Debug)]
struct AttributeInfo{
    attribute_name_index: ByteType,
    info: Vec<ByteType>
}

struct ClassFile{
    magic: ByteType,
    minor_version: ByteType,
    major_version: ByteType,
    constant_pool: Vec<CPInfo>,
    access_flags: ByteType,
    this_class: ByteType,
    super_class: ByteType,
    interfaces: Vec<ByteType>,
    fields: Vec<MemberInfo>,
    methods: Vec<MemberInfo>,
    attributes: Vec<AttributeInfo>
}

impl Debug for ClassFile{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassFile")
            .field("magic", &format_args!("{:x}", self.magic))
            .field("minor", &self.minor_version)
            .field("major", &self.major_version)
            .field("constant_pool", &format_args!("{:#?}", self.constant_pool))
            .field("access_flags", &format_args!("{:b}", self.access_flags))
            .field("this_class", &self.this_class)
            .field("super_class", &self.super_class)
            .field("interfaces", &format_args!("{:#?}", self.interfaces))
            .field("fields", &format_args!("{:#?}", self.fields))
            .field("methods", &format_args!("{:#?}", self.methods))
            .field("attributes", &format_args!("{:#?}", self.attributes))
            .finish()
    }
}

fn get_constant_printable(constant_pool: &Vec<CPInfo>, index: u32) -> String{
    let constant = constant_pool.get(index as usize).expect(format!("Constant at index {} not found", index).as_str());
    match constant.tag{
        Constant::Utf8 => {
            let bytes: Vec<u8> = constant.info.iter().map(|&e| if let U1(inner) = e {inner} else {0u8}).collect();
            String::from_utf8(bytes).expect("String could not be parsed")
        }
        Constant::Methodref | Constant::Fieldref => {
            let class_index: u32 = constant.info.get(0).expect("CONSTANT_Methodref or CONSTANT_Fieldref constant has wrong signature").clone().into();
            let name_and_type_index: u32 = constant.info.get(1).expect("CONSTANT_Methodref or CONSTANT_Fieldref constant has wrong signature").clone().into();
            format!("{}.{}", get_constant_printable(constant_pool, class_index-1), get_constant_printable(constant_pool, name_and_type_index-1))
        }
        Constant::NameAndType => {
            let name_index: u32 = constant.info.get(0).expect("CONSTANT_NameAndType constant has wrong signature").clone().into();
            let descriptor_index: u32 = constant.info.get(1).expect("CONSTANT_NameAndType constant has wrong signature").clone().into();
            format!("{} {}", get_constant_printable(constant_pool, name_index-1), get_constant_printable(constant_pool, descriptor_index-1))
        }
        Constant::Class => {
            let name_index: u32 = constant.info.get(0).expect("CONSTANT_Class constant has wrong signature").clone().into();
            format!("{}", get_constant_printable(constant_pool, name_index-1))
        }
        Constant::String => {
            let string_index: u32 = constant.info.get(0).expect("CONSTANT_String constant has wrong signature").clone().into();
            format!("{}", get_constant_printable(constant_pool, string_index-1))
        }
        _ => unimplemented!("Constant with type {:?} is not printable", constant.tag)
    }
}

fn get_attribute_printable(constant_pool: &Vec<CPInfo>, attributes: &Vec<AttributeInfo>, index: u32) -> String{
    let attribute = attributes.get(index as usize).expect(format!("Attribute at index {} not found", index).as_str());
    let name_index: u32 = attribute.attribute_name_index.into();
    let attribute_name = get_constant_printable(constant_pool, name_index-1);
    match attribute_name.as_str() {
        "SourceFile" => {
            let sourcefile_index: u32 = read_u2(&attribute.info, &mut 0).into();
            format!("{}: {}", attribute_name, get_constant_printable(constant_pool, sourcefile_index-1))
        }
        "Code" => {
            let mut offset = 0;
            let _max_stack = read_u2(&attribute.info, &mut offset);
            let _max_locals = read_u2(&attribute.info, &mut offset);
            let code_length: u32 = read_u4(&attribute.info, &mut offset).into();
            let mut code = Vec::new();
            for i in 0..code_length{
                let c = attribute.info.get((i  as usize) + offset).expect(format!("Code at index {} ({}) not found", i, i as usize + offset).as_str()).clone();
                code.push(format!("{:x}", c));
            }
            format!("Code: {:?}", code)
        }
        _ => attribute_name
    }
}

fn get_member_printable(constant_pool: &Vec<CPInfo>, members: &Vec<MemberInfo>, index: u32) -> String{
    let member = members.get(index as usize).expect(format!("Member at index {} not found", index).as_str());
    let name_index: u32 = member.name_index.into();
    let descriptor_index: u32 = member.descriptor_index.into();
    let mut attributes = String::new();
    attributes.push_str("\n");
    for i in 0..member.attributes.len(){
        attributes += format!("    [{}] {}\n", i+1, get_attribute_printable(constant_pool, &member.attributes, i as u32)).as_str();
    }
    format!("{}{} [{}]", get_constant_printable(constant_pool, name_index-1), get_constant_printable(constant_pool, descriptor_index-1), attributes)
}

fn parse_class_file(path: &str) -> std::io::Result<()> {
    let file = File::open(path)?;
    let mut bytes = file.bytes();

    let magic = parse_u4(&mut bytes)?;
    let minor_version = parse_u2(&mut bytes)?;
    let major_version = parse_u2(&mut bytes)?;
    let constant_pool_count: u32 = parse_u2(&mut bytes)?.into();
    let mut constant_pool = Vec::new();
    for _ in 0..constant_pool_count - 1{
        if let U1(raw_tag) = parse_u1(&mut bytes)?{
            let tag = Constant::from_repr(raw_tag).expect(format!("Could not get Constant of type {}", raw_tag).as_str());
            let info = match tag {
                Constant::Class => {
                    // name_index
                    vec![parse_u2(&mut bytes)?]
                }
                Constant::Fieldref | Constant::Methodref | Constant::InterfaceMethodref => {
                    // class_index, name_and_type_index
                    vec![parse_u2(&mut bytes)?, parse_u2(&mut bytes)?]
                }
                Constant::NameAndType => {
                    // name_index, descriptor_index
                    vec![parse_u2(&mut bytes)?, parse_u2(&mut bytes)?]
                }
                Constant::Utf8 => {
                    // name_index, descriptor_index
                    let mut info = Vec::new();
                    let length = parse_u2(&mut bytes)?;
                    //info.push(length);
                    let length: u32 = length.into();
                    for _ in 0..length{
                        info.push(parse_u1(&mut bytes)?)
                    }
                    info
                }
                Constant::String => {
                    // string_index
                    vec![parse_u2(&mut bytes)?]
                }
                _ => unimplemented!("CPTag {tag:?} not supported yet")
            };
            constant_pool.push(CPInfo{
                tag,
                info
            })
        } else {
            panic!("Tag could not be parsed")
        }
    }
    let access_flags = parse_u2(&mut bytes)?;
    let this_class = parse_u2(&mut bytes)?;
    let super_class = parse_u2(&mut bytes)?;
    let interfaces_count: u32 = parse_u2(&mut bytes)?.into();
    let mut interfaces = Vec::new();
    for _ in 0..interfaces_count{
        interfaces.push(parse_u2(&mut bytes)?);
    }
    let fields_count: u32 = parse_u2(&mut bytes)?.into();
    let mut fields = Vec::new();
    for _ in 0..fields_count{
        let access_flags = parse_u2(&mut bytes)?;
        let name_index = parse_u2(&mut bytes)?;
        let descriptor_index = parse_u2(&mut bytes)?;
        let attributes_count: u32 = parse_u2(&mut bytes)?.into();
        let mut attributes = Vec::new();
        for _ in 0..attributes_count{
            let attribute_name_index = parse_u2(&mut bytes)?;
            let attribute_length: u32 = parse_u4(&mut bytes)?.into();
            let mut info = Vec::new();
            for _ in 0..attribute_length{
                info.push(parse_u1(&mut bytes)?)
            }

            attributes.push(AttributeInfo{
                attribute_name_index,
                info
            });
        }
        fields.push(MemberInfo{
            access_flags,
            name_index,
            descriptor_index,
            attributes
        });
    }
    let method_count: u32 = parse_u2(&mut bytes)?.into();
    let mut methods = Vec::new();
    for _ in 0..method_count{
        let access_flags = parse_u2(&mut bytes)?;
        let name_index = parse_u2(&mut bytes)?;
        let descriptor_index = parse_u2(&mut bytes)?;
        let attributes_count: u32 = parse_u2(&mut bytes)?.into();
        let mut attributes = Vec::new();
        for _ in 0..attributes_count{
            let attribute_name_index = parse_u2(&mut bytes)?;
            let attribute_length: u32 = parse_u4(&mut bytes)?.into();
            let mut info = Vec::new();
            for _ in 0..attribute_length{
                info.push(parse_u1(&mut bytes)?)
            }

            attributes.push(AttributeInfo{
                attribute_name_index,
                info
            });
        }
        methods.push(MemberInfo{
            access_flags,
            name_index,
            descriptor_index,
            attributes
        });
    }
    let attributes_count: u32 = parse_u2(&mut bytes)?.into();
    let mut attributes = Vec::new();
    for _ in 0..attributes_count{
        let attribute_name_index = parse_u2(&mut bytes)?;
        let attribute_length: u32 = parse_u4(&mut bytes)?.into();
        let mut info = Vec::new();
        for _ in 0..attribute_length{
            info.push(parse_u1(&mut bytes)?)
        }

        attributes.push(AttributeInfo{
            attribute_name_index,
            info
        });
    }

    let class_file = ClassFile{
        magic,
        minor_version,
        major_version,
        constant_pool,
        access_flags,
        this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attributes
    };


    //println!("{:#?}", class_file);
    for i in 0..class_file.constant_pool.len(){
        println!("[{}] {} {:?}", i+1, get_constant_printable(&class_file.constant_pool, i as u32), &class_file.constant_pool.get(i).unwrap().tag);
    }

    for i in 0..class_file.attributes.len(){
        println!("[{}] {}", i+1, get_attribute_printable(&class_file.constant_pool, &class_file.attributes, i as u32));
    }

    for i in 0..class_file.methods.len(){
        println!("[{}] {}", i+1, get_member_printable(&class_file.constant_pool, &class_file.methods, i as u32));
    }
    // ["2a", "b7", "0", "1", "b1"]
    // aload_0
    // invokespecial x0001
    // return

    // ["b2", "0", "7", "12", "d", "b6", "0", "f", "b1"]
    // getstatic x0007
    // ldc 13
    // invokevirtual x000f
    // return

    for i in 0..class_file.fields.len(){
        println!("[{}] {}", i+1, get_member_printable(&class_file.constant_pool, &class_file.fields, i as u32));
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    parse_class_file("resources/Main.class")?;

    Ok(())
}
