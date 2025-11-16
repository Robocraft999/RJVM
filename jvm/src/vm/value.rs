use std::cell::{Ref, RefCell};
use std::fmt::{format, Debug, Display, Formatter, Pointer};
use crate::field_info::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;
use crate::vm::{VmError, VM};

#[derive(PartialEq, Default, Clone)]
pub enum Value<'a>{
    #[default]
    Uninitialized,
    Reference(Reference<'a>),

    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Dummy,

    Null,
}

impl Debug for Value<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Reference(rv) => write!(f,"{:?}", rv),
            Value::Uninitialized => write!(f, "VUninitialized"),
            Value::Null => write!(f, "VNull"),
            Value::Integer(value) => write!(f, "VInt ({})", value),
            Value::Long(value) => write!(f, "VLong ({})", value),
            Value::Float(value) => write!(f, "VFloat ({:.8})", value),
            Value::Double(value) => write!(f, "VDouble ({:.8})", value),
            Value::Dummy => write!(f, "VDummy")
        }
    }
}

impl<'a> Value<'a>{
    pub fn expect_int(&self) -> Result<i32, VmError> {
        if let Value::Integer(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected integer but found {:?}", self)))
        }
    }

    pub fn expect_long(&self) -> Result<i64, VmError> {
        if let Value::Long(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected long but found {:?}", self)))
        }
    }

    pub fn expect_float(&self) -> Result<f32, VmError> {
        if let Value::Float(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected float but found {:?}", self)))
        }
    }

    pub fn expect_double(&self) -> Result<f64, VmError> {
        if let Value::Double(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected double but found {:?}", self)))
        }
    }

    pub fn expect_reference(&self) -> Result<Reference<'a>, VmError> {
        if let Value::Reference(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected reference but found {:?}", self)))
        }
    }
    
    pub fn get_computational_type(&self) -> i32{
        match self {
            Value::Uninitialized => -1,
            Value::Reference(_) => 1,
            Value::Integer(_) => 1,
            Value::Long(_) => 2,
            Value::Float(_) => 1,
            Value::Double(_) => 2,
            Value::Dummy => -1,
            Value::Null => 1,
        }
    }
}

impl From<bool> for Value<'_>{
    fn from(value: bool) -> Self{
        Self::Integer(if value { 1 } else { 0 })
    }
}

pub type Reference<'a> = &'a ReferenceValue<'a>;

#[derive(PartialEq, Clone)]
pub struct ReferenceValue<'a>{
    pub(crate) id: u32,
    pub(crate) class_id: ClassId,
    pub(crate) class_name: String,
    pub(crate) reference_type: ReferenceType<'a>,
}

impl<'a> ReferenceValue<'a>{
    pub fn set_field(&self, index: usize, value: Value<'a>) {
        match &self.reference_type {
            ReferenceType::Object(fields) => {fields.borrow_mut()[index] = value}
            ReferenceType::Array(_, _, _) => {unimplemented!("This reference represents an array, please use 'set_element()'")}
        };
    }

    pub fn get_field(&self, index: usize) -> Value<'a>{
        match &self.reference_type {
            ReferenceType::Object(fields) => {fields.borrow()[index].clone()}
            ReferenceType::Array(_, _, _) => {unimplemented!("This reference represents an array, please use 'get_element()'")}
        }
    }

    pub fn set_element(&self, index: usize, value: Value<'a>) {
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'set_field()'")}
            ReferenceType::Array(_, _, content) => {content.borrow_mut()[index] = value}
        };
    }

    pub fn get_element(&self, index: usize) -> Value<'a>{
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(_, _, content) => {content.borrow()[index].clone()}
        }
    }

    pub fn get_length(&self) -> usize{
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(_, _, content) => {content.borrow().len()}
        }
    }

    pub fn is_array(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(_, _, _) => true,
            ReferenceType::Object(_) => false
        }
    }

    pub fn is_object(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(_, _, _) => false,
            ReferenceType::Object(_) => true
        }
    }

    fn get_components_printable(&self) -> Vec<String>{
        let object = |field: &Value| match field {
            Value::Reference(rv) => {
                if rv.class_name == "java/lang/String" {
                    format!("{}:{}:{:?}->'{}'", rv.id, rv.class_name, rv.class_id.0, VM::extract_string_from_object(field).unwrap_or("VMError".to_string()))
                } else {
                    format!("{}:{}:{:?}", rv.id, rv.class_name, rv.class_id.0)
                }
            }
            _ => format!("{:?}", field)
        };
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                if self.class_name == "java/lang/String" {
                    let internal = VM::extract_string_from_char_arr(&self.get_field(0)).unwrap_or("VMError".to_string());
                    let mut components = Vec::new();
                    components.push(internal);
                    let mut other_fields = fields.borrow().iter().skip(1).map(object).collect();
                    components.append(&mut other_fields);
                    components
                } else {
                    fields.borrow().iter().map(object).collect()
                }
            },
            ReferenceType::Array(_, field_type, content) => {
                if let FieldType::Primitive(PrimitiveType::Char) = field_type {
                    let chars: Vec<char> = content.borrow().iter().map(|e| if let Value::Integer(val) = e {char::from_u32(*val as u32).unwrap()} else {'?'}).collect();
                    vec![chars.iter().collect::<String>()]
                } else {
                    let mut vec = Vec::new();
                    let mut null_counter = 0;
                    for value in content.borrow().iter(){
                        if let Value::Null = value{
                            null_counter += 1;
                        } else {
                            if null_counter > 0{
                                vec.push(format!("{}x{}", null_counter, object(&Value::Null)));
                                null_counter = 0;
                            }
                            vec.push(object(&value));
                        }
                    }
                    if null_counter > 0{
                        vec.push(format!("{}x{}", null_counter, object(&Value::Null)));
                    }
                    vec
                }
            }
        }
    }
}

impl Debug for ReferenceValue<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VRef")
            .field("object_id", &self.id)
            .field("class", &format_args!("{}:{}", &self.class_name, &self.class_id.0))
            .field("type", &match self.reference_type {
                ReferenceType::Object(_) => "Object",
                ReferenceType::Array(_, _, _) => "Array",
            })
            .field("components", &self.get_components_printable())
            .finish()
    }
}

#[derive(PartialEq, Clone)]
pub enum ReferenceType<'a>{
    Object(RefCell<Vec<Value<'a>>>),
    Array(usize, FieldType, RefCell<Vec<Value<'a>>>)
}