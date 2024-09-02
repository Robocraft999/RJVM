use std::fmt::Debug;

use strum_macros::FromRepr;

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
    MethodHandle = 15,
    MethodType = 16,
    InvokeDynamic(u16, u16) = 18,
    Dummy = 255,
}

#[derive(Debug)]
pub struct ConstantPool(pub Vec<ConstantPoolEntry>);