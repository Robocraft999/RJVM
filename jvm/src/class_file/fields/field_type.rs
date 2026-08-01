use crate::class_file::fields::{primitive_type_to_class_name, primitive_type_to_descriptor};
use crate::vm::result::VMResult;
use crate::vm::value::Value;
use crate::vm::VmError;
use lazy_regex::{lazy_regex, regex, Lazy};
use regex::Regex;
use std::str::FromStr;

static PATTERN: Lazy<Regex> = lazy_regex!(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9_]+);)");

pub fn get_field_type_raw_parts(raw: &str) -> VMResult<(Option<&str>, Option<&str>, Option<&str>)> {
    if let Some(cap) = PATTERN.captures(raw){
        Ok((cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), cap.name("array").map(|m| m.as_str())))
    } else {
        Err(VmError::ValidationError(format!("{} is not a valid field type", raw)))
    }
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
    Array(String, Box<FieldType>),
}

impl FieldType {
    pub(crate) fn to_class_name(&self) -> String {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type_to_class_name(primitive_type),
            FieldType::Object(name) => format!("{}", name),
            FieldType::Array(name, _) => format!("{}", name),
        }
    }

    pub fn to_descriptor(&self) -> String {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type_to_descriptor(primitive_type),
            FieldType::Object(name) => format!("L{};", name),
            FieldType::Array(name, _) => format!("{}", name),
        }
    }

    pub fn to_array_field_type(self, dims: usize) -> FieldType{
        if dims == 0{
            panic!("Can't make {self:?} an array type because dims is 0");
        }
        let prefix = "[".repeat(dims);
        match self.clone() {
            FieldType::Primitive(primitive_type) => {
                let name = prefix + primitive_type_to_descriptor(&primitive_type).as_str();
                FieldType::Array(name, Box::new(self))
            }
            FieldType::Object(name) => {
                let name = prefix + "L" + name.as_str() + ";";
                FieldType::Array(name, Box::new(self))
            }
            //FIXME should we allow this?
            FieldType::Array(_, _) => panic!("Can't make {self:?} an array type, because it is already one"),
        }
    }

    pub fn get_locals_length(&self) -> usize{
        match self{
            FieldType::Object(_) => 1,
            FieldType::Array(_, _) => 1,
            FieldType::Primitive(PrimitiveType::Boolean) => 1,
            FieldType::Primitive(PrimitiveType::Byte) => 1,
            FieldType::Primitive(PrimitiveType::Char) => 1,
            FieldType::Primitive(PrimitiveType::Short) => 1,
            FieldType::Primitive(PrimitiveType::Integer) => 1,
            FieldType::Primitive(PrimitiveType::Long) => 2,
            FieldType::Primitive(PrimitiveType::Float) => 1,
            FieldType::Primitive(PrimitiveType::Double) => 2,
        }
    }

    pub fn get_default_value<'a>(&self, null: Value) -> Value {
        match self {
            FieldType::Primitive(primitive) => {
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
            FieldType::Object(_) => null,
            FieldType::Array(_, _) => null,
        }
    }

    pub fn from_raw_parts(object: Option<&str>, primitive: Option<&str>, array: Option<&str>) -> VMResult<FieldType>{
        let field_type = if let Some(obj) = object{
            Some(FieldType::Object(String::from(obj)))
        }else if let Some(prim) = primitive{
            Some(FieldType::Primitive(PrimitiveType::from_str(prim)?))
        } else {
            None
        };
        if let Some(dims_amount_of_brackets) = array{
            let mut name = String::new();
            name.push_str(dims_amount_of_brackets);
            if let Some(obj) = object{
                name.push_str("L");
                name.push_str(obj);
                name.push_str(";");
            }
            if let Some(prim) = primitive{
                name.push_str(prim);
            }
            let component_type = field_type.ok_or_else(|| VmError::ValidationError(format!("{} is neither object nor primitive field type", name)))?;
            Ok(FieldType::Array(name, Box::from(component_type)))
        } else {
            field_type.ok_or(VmError::ValidationError("Field type is neither object nor primitive".to_owned()))
        }
    }
    
    pub fn is_primitive(&self) -> bool {
        if let FieldType::Primitive(_) = self {
            true
        } else {
            false
        }
    }
}

impl FromStr for FieldType{
    type Err = VmError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (object, primitive, array) = get_field_type_raw_parts(s)?;
        FieldType::from_raw_parts(object, primitive, array)
    }
}

impl PartialEq<Value> for FieldType{
    fn eq(&self, other: &Value) -> bool {
        match (other, self) {
            (Value::Reference(..), FieldType::Object(..)) | (Value::Reference(..), FieldType::Array(..)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Integer)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Short)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Byte)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Boolean)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Char)) => true,
            (Value::Long(..), FieldType::Primitive(PrimitiveType::Long)) => true,
            (Value::Float(..), FieldType::Primitive(PrimitiveType::Float)) => true,
            (Value::Double(..), FieldType::Primitive(PrimitiveType::Double)) => true,
            _ => false
        }
    }
}


#[derive(Debug, PartialEq, Clone)]
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
            _ => Err(VmError::ValidationError(format!("Invalid primitive type {}", s)))
        }
    }
}