use std::str::FromStr;
use regex::Regex;
use crate::access_flags::FieldFlags;
use crate::attribute::{Attribute, ConstantValue};

#[derive(Debug)]
pub struct FieldInfo{
    pub flags: FieldFlags,
    pub name: String,
    pub field_type: FieldType,
    pub deprecated: bool,
    pub constant_value: Option<ConstantValue>,
    pub attributes: Vec<Attribute>
}

pub fn field_type_from_str(string: &str) -> FieldType{
    let r = Regex::new(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$]+);)").unwrap();
    if let Some(cap) = r.captures(string){
        parse_field_type(cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), cap.name("array").map(|m| m.len()))
    } else {
        panic!("Expected a fieldtype which could not be parsed from {string}")
    }
}


//FIXME not sure about this tbo
pub fn parse_field_type(object: Option<&str>, primitive: Option<&str>, array_dims: Option<usize>) -> FieldType{
    let field_type = if let Some(prim) = primitive{
        FieldType::Primitive(PrimitiveType::from_str(prim).unwrap())
    } else if let Some(obj) = object{
        FieldType::Object(String::from(obj))
    } else {
        unreachable!("Type is neither object nor primitive")
    };

    if let Some(dims) = array_dims{
        FieldType::Array(dims, Box::new(field_type))
    } else {
        field_type
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType{
    Primitive(PrimitiveType),
    Object(String),
    Array(usize, Box<FieldType>),
}

#[derive(Debug, PartialEq, Clone)]
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