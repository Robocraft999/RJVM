use crate::class_file::attributes::BootstrapMethod;
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::class_file::nom::parse_cesu_string;
use crate::vm::class::class_and_member::{ClassAndField, ClassAndMethod};
use crate::vm::class::ClassRef;
use nom::combinator::map;
use nom::error::Error;
use nom::number::{be_f32, be_f64, be_i32, be_i64, be_u16, be_u8};
use nom::{IResult, Parser};
use nom_derive::Parse;
use strum_macros::FromRepr;

#[derive(Debug, Clone)]
pub enum ConstantPoolEntry<'a>{
    RawClass(u16),
    Class(ClassRef<'a>),

    RawFieldRef(u16, u16),
    FieldRef(ClassAndField<'a>),

    RawMethodRef(u16, u16),
    MethodRef(ClassAndMethod<'a>),
    MethodRefSigPoly(ClassAndMethod<'a>, MethodDescriptor),

    RawInterfaceMethodRef(u16, u16),
    InterfaceMethodRef(ClassAndMethod<'a>),

    RawString(u16),
    String(String),

    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),

    RawNameAndType(u16, u16),
    NameAndType(String, String),

    Utf8(String),

    RawMethodHandle(u8, u16),
    MethodHandleField(BytecodeBehavior, ClassAndField<'a>),
    MethodHandleMethod(BytecodeBehavior, ClassAndMethod<'a>),

    RawMethodType(u16),
    MethodType(MethodDescriptor),

    RawInvokeDynamic(u16, u16),
    InvokeDynamic(BootstrapMethod, String, String),

    Dummy
}

impl Parse<&[u8]> for ConstantPoolEntry<'_> {

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
            7  => map(be_u16(), ConstantPoolEntry::RawClass).parse(remaining),
            8  => map(be_u16(), ConstantPoolEntry::RawString).parse(remaining),
            9  => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::RawFieldRef(a, b)).parse(remaining),
            10 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::RawMethodRef(a, b)).parse(remaining),
            11 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::RawInterfaceMethodRef(a, b)).parse(remaining),
            12 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::RawNameAndType(a, b)).parse(remaining),
            15 => map((be_u8() , be_u16()), |(a, b)| ConstantPoolEntry::RawMethodHandle(a, b)).parse(remaining),
            16 => map(be_u16(), ConstantPoolEntry::RawMethodType).parse(remaining),
            18 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::RawInvokeDynamic(a, b)).parse(remaining),
            _ => Err(nom::Err::Error(Error::new(remaining, nom::error::ErrorKind::Switch))),
        }
    }
}

pub type ConstantPool<'a> = Vec<ConstantPoolEntry<'a>>;

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