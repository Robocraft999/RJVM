use std::fmt::Debug;

use strum_macros::FromRepr;
use crate::attribute::BootstrapMethod;
use crate::method_info::{MethodDescriptor};
use crate::vm::class::{ClassAndField, ClassAndMethod, ClassRef};

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