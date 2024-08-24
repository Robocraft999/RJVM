mod constants;
mod bytes;
mod access_flags;
mod attribute;
mod class_file_version;
mod field_info;
mod method_info;

use std::fmt::{format, write, Debug, Formatter};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use access_flags::{parse_class_flags, parse_field_flags, parse_method_flags};
use attribute::{Attribute, Code};
use bytes::{read_u2, read_u4};

use crate::access_flags::ClassFlags;
use crate::bytes::{ByteType, parse_u1, parse_u2, parse_u4};
use crate::constants::*;
use crate::class_file_version::ClassFileVersion;
use crate::field_info::FieldInfo;
use crate::method_info::MethodInfo;

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

fn get_constant_printable(constant_pool: &ConstantPool, index: u16) -> String{
    let constant = constant_pool.0.get(index as usize - 1).expect(format!("Constant at index {} not found", index -1).as_str());
    match constant.clone(){
        ConstantPoolEntry::Utf8(string) => {
            string.to_string()
        }
        ConstantPoolEntry::Methodref(class_index, name_and_type_index) | ConstantPoolEntry::Fieldref(class_index, name_and_type_index) => {
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
        _ => unimplemented!("Constant with type {:?} is not printable", constant)
    }
}

fn get_attribute_printable(constant_pool: &ConstantPool, attributes: &Vec<Attribute>, index: u32) -> String{
    let attribute = attributes.get(index as usize).expect(format!("Attribute at index {} not found", index).as_str());
    let attribute_name = &attribute.name;
    match attribute_name.as_str() {
        /*"SourceFile" => {
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
        }*/
        _ => attribute_name.to_string()
    }
}

/*fn get_member_printable(constant_pool: &ConstantPool, members: &Vec<MemberInfo>, index: u16) -> String{
    let member = members.get(index as usize).expect(format!("Member at index {} not found", index).as_str());
    let name_index = member.name_index;
    let descriptor_index = member.descriptor_index;
    let mut attributes = String::new();
    attributes.push_str("\n");
    for i in 0..member.attributes.len(){
        attributes += format!("    [{}] {}\n", i+1, get_attribute_printable(constant_pool, &member.attributes, i as u32)).as_str();
    }
    format!("{}{} [{}]", get_constant_printable(constant_pool, name_index), get_constant_printable(constant_pool, descriptor_index), attributes)
}*/

fn parse_class_file(path: &str) -> std::io::Result<()> {
    let file = File::open(path)?;
    let mut bytes = file.bytes();

    let magic = parse_u4(&mut bytes)?;
    let _minor_version = parse_u2(&mut bytes)?;
    let major_version = parse_u2(&mut bytes)?;
    let class_file_version = ClassFileVersion::from_repr(major_version).expect(format!("Could not parse ClassFileVersion {}", major_version).as_str());
    let constant_pool_count: u32 = parse_u2(&mut bytes)?.into();
    let mut constant_pool_entries = Vec::new();
    for _ in 0..constant_pool_count - 1{
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
            _ => unimplemented!("CPTag {tag:?} not supported yet")
        };
        constant_pool_entries.push(constant_pool_entry);
    }
    let constant_pool = ConstantPool(constant_pool_entries);
    let access_flags = parse_class_flags(parse_u2(&mut bytes)?);
    let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
    let super_class = Some(get_constant_printable(&constant_pool, parse_u2(&mut bytes)?));
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
        let attributes_count = parse_u2(&mut bytes)?;
        let mut attributes = Vec::new();
        let mut deprecated = false;
        let mut constant_value = None;
        for _ in 0..attributes_count{
            let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
            let attribute_length = parse_u4(&mut bytes)?;
            let mut info = Vec::new();
            for _ in 0..attribute_length{
                info.push(parse_u1(&mut bytes)?)
            }

            attributes.push(Attribute{
                name,
                info
            });
        }
        fields.push(FieldInfo{
            flags,
            name,
            descriptor,
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
        let descriptor = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
        let attributes_count = parse_u2(&mut bytes)?;
        let mut attributes = Vec::new();
        let mut deprecated = false;
        let mut code = None;
        dbg!(&flags, &name, &descriptor, &attributes_count);
        for _ in 0..attributes_count{
            let name = get_constant_printable(&constant_pool, parse_u2(&mut bytes)?);
            let attribute_length = parse_u4(&mut bytes)?;
            dbg!(&name, &attribute_length);

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
                        let mut info = Vec::new();
                        for _ in 0..attribute_length{
                            info.push(parse_u1(&mut bytes)?)
                        }
                        code_attributes.push(Attribute{
                            name,
                            info
                        })
                    }

                    code = Some(Code{
                        max_stack,
                        max_locals,
                        code: code_bytes,
                        attributes: code_attributes
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
            println!("- - - - - - - -");
        }
        methods.push(MethodInfo{
            flags,
            name,
            descriptor,
            deprecated,
            attributes,
            code
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
        println!("[{}] {} {:?}", i+1, get_constant_printable(&class_file.constant_pool, i as u16 + 1), &class_file.constant_pool.0.get(i).unwrap());
    }

    /*for i in 0..class_file.methods.len(){
        println!("[{}] {}", i+1, get_member_printable(&class_file.constant_pool, &class_file.methods, i as u32));
    }*/
    // ["2a", "b7", "0", "1", "b1"]
    // aload_0
    // invokespecial x0001
    // return

    // ["b2", "0", "7", "12", "d", "b6", "0", "f", "b1"]
    // getstatic x0007
    // ldc 13
    // invokevirtual x000f
    // return

    /*for i in 0..class_file.fields.len(){
        println!("[{}] {}", i+1, get_member_printable(&class_file.constant_pool, &class_file.fields, i as u32));
    }*/

    Ok(())
}

fn main() -> std::io::Result<()> {
    parse_class_file("resources/Main.class")?;

    Ok(())
}
