use std::fmt::Debug;
use strum_macros::FromRepr;

#[derive(Debug, FromRepr, PartialEq)]
#[repr(u8)]
pub enum Constant{
    Class = 7,
    Fieldref = 9,
    Methodref = 10,
    InterfaceMethodref = 11,
    String = 8,
    Integer = 3,
    Float = 4,
    Long = 5,
    Double = 6,
    NameAndType = 12,
    Utf8 = 1,
    MethodHandle = 15,
    MethodType = 16,
    InvokeDynamic = 18
}

pub const ACC_PUBLIC: u16     = 0x0001; //000000000000001
pub const ACC_FINAL: u16      = 0x0010; //000000000010000
pub const ACC_SUPER: u16      = 0x0020; //000000000100000
pub const ACC_INTERFACE: u16  = 0x0200; //000001000000000
pub const ACC_ABSTRACT: u16   = 0x0400; //000010000000000
pub const ACC_SYNTHETIC: u16  = 0x1000; //001000000000000
pub const ACC_ANNOTATION: u16 = 0x2000; //010000000000000
pub const ACC_ENUM: u16       = 0x4000; //100000000000000