use crate::class_file::fields::{primitive_type_to_class_name, primitive_type_to_descriptor};
use crate::vm::VmError;
use crate::vm::result::VMResult;
use crate::vm::value::Value;
use lazy_regex::{Lazy, lazy_regex, regex};
use regex::Regex;
use std::str::FromStr;

static PATTERN: Lazy<Regex> = lazy_regex!(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9_]+);)");

pub fn get_field_type_raw_parts(raw: &str) -> VMResult<(Option<&str>, Option<&str>, Option<&str>)> {
    PATTERN
        .captures(raw)
        .map_or_else(||
                         Err(VmError::ValidationError(format!("{raw} is not a valid field type"))),
                     |cap|
                         Ok((
                             cap.name("object").map(|m| m.as_str()),
                             cap.name("primitive").map(|m| m.as_str()),
                             cap.name("array").map(|m| m.as_str()))
                         )
        )
}

pub fn extract_component_type_from_array_class(array_class_descriptor: &str) -> VMResult<(FieldType, usize)> {
    let (object, primitive, array) = get_field_type_raw_parts(array_class_descriptor)?;
    let array_type = FieldType::from_raw_parts(object, primitive, array)?;
    if let FieldType::Array(_, component_type) = array_type {
        Ok((*component_type, array.unwrap_or("").len()))
    } else {
        Err(VmError::ValidationError("Can't extract component type from non-array type".to_owned()))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType{
    Primitive(PrimitiveType),
    Object(String),
    Array(String, Box<Self>),
}

impl FieldType {
    pub(crate) fn to_class_name(&self) -> String {
        match self {
            Self::Primitive(primitive_type) => primitive_type_to_class_name(primitive_type),
            Self::Object(name) | Self::Array(name, _) => name.to_owned(),
        }
    }

    pub fn to_descriptor(&self) -> String {
        match self {
            Self::Primitive(primitive_type) => primitive_type_to_descriptor(primitive_type),
            Self::Object(name) => format!("L{name};"),
            Self::Array(name, _) => name.clone(),
        }
    }

    pub fn to_array_field_type(&self, dims: usize) -> Self{
        assert_ne!(dims, 0, "Can't make {self:?} an array type because dims is 0");
        let prefix = "[".repeat(dims);
        match self {
            // Extends the primitive type to an n-dim array type
            Self::Primitive(primitive_type) => {
                let name = prefix + primitive_type_to_descriptor(primitive_type).as_str();
                Self::Array(name, Box::new(self.clone()))
            }
            // Extends the object type to an n-dim array type
            Self::Object(name) => {
                let name = prefix + "L" + name.as_str() + ";";
                Self::Array(name, Box::new(self.clone()))
            }
            // Extends the m-dim array type to an (n+m)-dim array type
            Self::Array(name, component_type) => Self::Array(prefix + name.as_str(), component_type.clone()),
        }
    }

    pub const fn get_locals_length(&self) -> usize{
        match self{
            Self::Object(_) => 1,
            Self::Array(_, _) => 1,
            Self::Primitive(PrimitiveType::Boolean) => 1,
            Self::Primitive(PrimitiveType::Byte) => 1,
            Self::Primitive(PrimitiveType::Char) => 1,
            Self::Primitive(PrimitiveType::Short) => 1,
            Self::Primitive(PrimitiveType::Integer) => 1,
            Self::Primitive(PrimitiveType::Long) => 2,
            Self::Primitive(PrimitiveType::Float) => 1,
            Self::Primitive(PrimitiveType::Double) => 2,
        }
    }

    pub const fn get_default_value(&self, null: Value) -> Value {
        match self {
            Self::Primitive(primitive) => {
                match primitive {
                    PrimitiveType::Boolean => Value::Integer(0),
                    PrimitiveType::Byte => Value::Integer(0),
                    PrimitiveType::Char => Value::Integer(0),
                    PrimitiveType::Short => Value::Integer(0),
                    PrimitiveType::Integer => Value::Integer(0),
                    PrimitiveType::Long => Value::Long(0),
                    PrimitiveType::Float => Value::Float(0f32),
                    PrimitiveType::Double => Value::Double(0f64),
                }
            }
            Self::Object(_) => null,
            Self::Array(_, _) => null,
        }
    }

    pub fn from_raw_parts(object: Option<&str>, primitive: Option<&str>, array: Option<&str>) -> VMResult<Self>{
        let field_type = if let Some(obj) = object{
            Some(Self::Object(String::from(obj)))
        }else if let Some(prim) = primitive{
            Some(Self::Primitive(PrimitiveType::from_str(prim)?))
        } else {
            None
        };
        if let Some(dims_amount_of_brackets) = array{
            let mut name = String::new();
            name.push_str(dims_amount_of_brackets);
            if let Some(obj) = object{
                name.push('L');
                name.push_str(obj);
                name.push(';');
            }
            if let Some(prim) = primitive{
                name.push_str(prim);
            }
            let component_type = field_type.ok_or_else(|| VmError::ValidationError(format!("{name} is neither object nor primitive field type")))?;
            Ok(Self::Array(name, Box::from(component_type)))
        } else {
            field_type.ok_or_else(|| VmError::ValidationError("Field type is neither object nor primitive".to_owned()))
        }
    }
    
    pub const fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_))
    }
}

impl FromStr for FieldType{
    type Err = VmError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (object, primitive, array) = get_field_type_raw_parts(s)?;
        Self::from_raw_parts(object, primitive, array)
    }
}

impl PartialEq<Value> for FieldType{
    fn eq(&self, other: &Value) -> bool {
        matches!((other, self),
            (Value::Reference(..), Self::Object(..)) |
            (Value::Reference(..), Self::Array(..)) |
            (Value::Integer(..),   Self::Primitive(PrimitiveType::Integer)) |
            (Value::Integer(..),   Self::Primitive(PrimitiveType::Short)) |
            (Value::Integer(..),   Self::Primitive(PrimitiveType::Byte)) |
            (Value::Integer(..),   Self::Primitive(PrimitiveType::Boolean)) |
            (Value::Integer(..),   Self::Primitive(PrimitiveType::Char)) |
            (Value::Long(..),      Self::Primitive(PrimitiveType::Long)) |
            (Value::Float(..),     Self::Primitive(PrimitiveType::Float)) |
            (Value::Double(..),    Self::Primitive(PrimitiveType::Double)))
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PrimitiveType{
    Boolean,
    Byte,
    Char,
    Double,
    Float,
    Integer,
    Long,
    Short,
}

impl FromStr for PrimitiveType{
    type Err = VmError;

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
            _ => Err(VmError::ValidationError(format!("Invalid primitive type {s}")))
        }
    }
}