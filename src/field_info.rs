use std::str::FromStr;
use regex::Regex;
use crate::access_flags::FieldFlags;
use crate::attribute::{Attribute, ConstantValue};
use crate::vm::value::Value;

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
    let r = Regex::new(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9]+);)").unwrap();
    if let Some(cap) = r.captures(string){
        parse_field_type(cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), cap.name("array").map(|m| m.as_str()))
    } else {
        panic!("Expected a fieldtype which could not be parsed from {string}")
    }
}


//FIXME not sure about this tbo
pub fn parse_field_type(object: Option<&str>, primitive: Option<&str>, array: Option<&str>) -> FieldType{
    let field_type = if let Some(prim) = primitive{
        FieldType::Primitive(PrimitiveType::from_str(prim).unwrap())
    } else if let Some(obj) = object{
        FieldType::Object(String::from(obj))
    } else {
        unreachable!("Type is neither object nor primitive")
    };

    if let Some(dims) = array{
        let mut name = String::new();
        name.push_str(dims);
        if let Some(obj) = object{
            name.push_str(obj);
            name.push_str(";");
        }
        if let Some(prim) = primitive{
            name.push_str(prim);
        }
        FieldType::Object(name)
    } else {
        field_type
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType{
    Primitive(PrimitiveType),
    Object(String),
}

impl FieldType {
    pub(crate) fn to_class_name(&self) -> String {
        match self {
            FieldType::Primitive(_) => self.get_primitive_class(),
            FieldType::Object(name) => format!("{}", name),
        }
    }

    pub fn get_primitive_class(&self) -> String {
        if let FieldType::Primitive(p_type) = self{
            match p_type {
                PrimitiveType::Integer => "java/lang/Integer".to_string(),
                PrimitiveType::Long    => "java/lang/Long".to_string(),
                PrimitiveType::Short   => "java/lang/Short".to_string(),
                PrimitiveType::Char    => "java/lang/Character".to_string(),
                PrimitiveType::Byte    => "java/lang/Byte".to_string(),
                PrimitiveType::Float   => "java/lang/Float".to_string(),
                PrimitiveType::Double  => "java/lang/Double".to_string(),
                PrimitiveType::Boolean => "java/lang/Boolean".to_string(),
            }
        } else {
            unreachable!("Type is not primitive")
        }
    }
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