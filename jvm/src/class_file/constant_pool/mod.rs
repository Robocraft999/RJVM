use nom::combinator::map;
use nom::error::Error;
use nom::{IResult, Parser};
use nom::number::{be_f32, be_f64, be_i32, be_i64, be_u16, be_u8};
use nom_derive::Parse;
use crate::attribute::BootstrapMethod;
use crate::class_file::method_info::MethodDescriptor;
use crate::vm::class::{ClassAndField, ClassAndMethod, ClassRef};
use strum_macros::FromRepr;
use crate::class_file::nom::parse_cesu_string;

#[derive(Debug, FromRepr, PartialEq, Clone)]
#[repr(u8)]
pub enum ConstantPoolEntry{
    Class(u16) = 7,
    Fieldref(u16, u16) = 9,
    Methodref(u16, u16) = 10,
    InterfaceMethodref(u16, u16) = 11,
    String(u16) = 8,
    Integer(i32) = 3,
    Float(f32) = 4,
    Long(i64) = 5,
    Double(f64) = 6,
    NameAndType(u16, u16) = 12,
    Utf8(String) = 1,
    MethodHandle(u8, u16) = 15,
    MethodType(u16) = 16,
    InvokeDynamic(u16, u16) = 18,
    Dummy = 255,
}

#[derive(Debug)]
pub struct ConstantPool(pub Vec<ConstantPoolEntry>);

impl Parse<&[u8]> for ConstantPoolEntry {

    fn parse(i: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        Self::parse_be(i)
    }

    fn parse_be(i: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (remaining, tag) = be_u8().parse(i)?;
        match tag {
            1  => map(parse_cesu_string, ConstantPoolEntry::Utf8).parse(remaining),
            3  => map(be_i32(), ConstantPoolEntry::Integer).parse(remaining),
            4  => map(be_f32(), ConstantPoolEntry::Float).parse(remaining),
            5  => map(be_i64(), ConstantPoolEntry::Long).parse(remaining),
            6  => map(be_f64(), ConstantPoolEntry::Double).parse(remaining),
            7  => map(be_u16(), ConstantPoolEntry::Class).parse(remaining),
            8  => map(be_u16(), ConstantPoolEntry::String).parse(remaining),
            9  => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::Fieldref(a, b)).parse(remaining),
            10 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::Methodref(a, b)).parse(remaining),
            11 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::InterfaceMethodref(a, b)).parse(remaining),
            12 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::NameAndType(a, b)).parse(remaining),
            15 => map((be_u8() , be_u16()), |(a, b)| ConstantPoolEntry::MethodHandle(a, b)).parse(remaining),
            16 => map(be_u16(), ConstantPoolEntry::MethodType).parse(remaining),
            18 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::InvokeDynamic(a, b)).parse(remaining),
            _ => Err(nom::Err::Error(Error::new(remaining, nom::error::ErrorKind::Alt))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FastConstantPoolEntry<'a>{
    RawClass(String),
    Class(ClassRef<'a>),

    RawFieldRef(String, String, String),
    FieldRef(ClassAndField<'a>),

    RawMethodRef(String, String, String),
    MethodRef(ClassAndMethod<'a>),

    RawInterfaceMethodRef(String, String, String),
    InterfaceMethodRef(ClassAndMethod<'a>),

    String(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),

    NameAndType(String, String),

    Utf8(String),

    RawMethodHandle(BytecodeBehavior, String, String, String),
    MethodHandleField(BytecodeBehavior, ClassAndField<'a>),
    MethodHandleMethod(BytecodeBehavior, ClassAndMethod<'a>),

    MethodType(MethodDescriptor),

    InvokeDynamic(BootstrapMethod, String, String),

    Dummy
}

pub type FastConstantPool<'a> = Vec<FastConstantPoolEntry<'a>>;

#[derive(Debug, FromRepr, PartialEq, Clone)]
#[repr(u8)]
pub enum BytecodeBehavior {
    REFGetField = 1,
    REFGetStatic = 2,
    REFPutField = 3,
    REFPutStatic = 4,
    REFInvokeVirtual = 5,
    REFInvokeStatic = 6,
    REFInvokeSpecial = 7,
    REFNewInvokeSpecial = 8,
    REFInvokeInterface = 9,
}