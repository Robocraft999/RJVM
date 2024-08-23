mod constants;
mod bytes;

use std::fmt::{Debug, Formatter, write};
use std::fs::File;
use std::io::Read;
use crate::bytes::{ByteType, parse_u1, parse_u2, parse_u4};
use crate::bytes::ByteType::{U1, U2, U4};
use crate::constants::*;

#[derive(Debug)]
struct CPInfo{
    tag: u8,
    info: Vec<ByteType>
}

struct ClassFile{
    magic: ByteType,
    minor_version: ByteType,
    major_version: ByteType,
    constant_pool: Vec<CPInfo>,
    access_flags: ByteType
}

impl Debug for ClassFile{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassFile")
            .field("magic", &format_args!("{:x}", self.magic))
            .field("minor", &self.minor_version)
            .field("major", &self.major_version)
            .field("constant_pool", &format_args!("{:#?}", self.constant_pool))
            .field("access_flags", &format_args!("{:b}", self.access_flags))
            .finish()
    }
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
        if let U1(tag) = parse_u1(&mut bytes)?{
            let mut info = vec![];
            if tag == CONSTANT_Class{
                // class_index
                info = vec![parse_u2(&mut bytes)?]
            } else if tag == CONSTANT_Fieldref || tag == CONSTANT_Methodref || tag == CONSTANT_InterfaceMethodref{
                // class_index, name_and_type_index
                info = vec![parse_u2(&mut bytes)?, parse_u2(&mut bytes)?]
            } else if tag == CONSTANT_NameAndType {
                // name_index, descriptor_index
                info = vec![parse_u2(&mut bytes)?, parse_u2(&mut bytes)?]
            } else if tag == CONSTANT_Utf8 {
                // name_index, descriptor_index
                info = Vec::new();
                let length = parse_u2(&mut bytes)?;
                info.push(length);
                let length: u32 = length.into();
                for _ in 0..length{
                    info.push(parse_u1(&mut bytes)?)
                }
            } else if tag == CONSTANT_String{
                // string_index
                info = vec![parse_u2(&mut bytes)?]
            } else {
                unimplemented!("CPTag {tag} not supported yet");
            }
            constant_pool.push(CPInfo{
                tag,
                info
            })
        } else {
            panic!("Tag could not be parsed")
        }
    }
    let access_flags = parse_u2(&mut bytes)?;

    let class_file = ClassFile{
        magic,
        minor_version,
        major_version,
        constant_pool,
        access_flags
    };


    println!("{:#?}", class_file);

    Ok(())
}

fn main() -> std::io::Result<()> {
    parse_class_file("resources/Main.class")?;

    Ok(())
}
