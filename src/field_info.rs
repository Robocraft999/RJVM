use std::str::FromStr;

use crate::access_flags::FieldFlags;
use crate::attribute::{Attribute, ConstantValue};

#[derive(Debug)]
pub struct FieldInfo{
    pub flags: FieldFlags,
    pub name: String,
    pub descriptor: String,
    pub deprecated: bool,
    pub constant_value: Option<ConstantValue>,
    pub attributes: Vec<Attribute>
}

#[derive(Debug)]
pub enum FieldType{
    Primitive(PrimitiveType),
    Object(String),
    Array(usize, Box<FieldType>),
}

#[derive(Debug)]
pub enum PrimitiveType{
    Boolean,
    Byte,
    Short,
    Integer,
    Long,
    Float,
    Double,
    Char
}

impl FromStr for PrimitiveType{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Z" => Ok(Self::Boolean),
            "B" => Ok(Self::Byte),
            "S" => Ok(Self::Short),
            "I" => Ok(Self::Integer),
            "J" => Ok(Self::Long),
            "F" => Ok(Self::Float),
            "D" => Ok(Self::Double),
            "C" => Ok(Self::Char),
            _   => unreachable!()
        }
    }
}