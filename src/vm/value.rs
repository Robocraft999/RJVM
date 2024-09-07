use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::ops::Index;
use std::rc::Rc;
use crate::field_info::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;

#[derive(PartialEq, Default, Clone)]
pub enum Value<'a>{
    #[default]
    Uninitialized,
    Reference(Reference<'a>),

    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),

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
        }
    }
}

pub type Reference<'a> = &'a ReferenceValue<'a>;

#[derive(PartialEq, Clone)]
pub struct ReferenceValue<'a>{
    pub(crate) id: u32,
    pub(crate) class_id: ClassId,
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

    fn get_components_printable(&self) -> Vec<String>{
        let object = |field: &Value| match field {
            Value::Reference(rv) => format!("{}:{:?}", rv.id, rv.class_id),
            _ => format!("{:?}", field)
        };
        match &self.reference_type {
            ReferenceType::Object(fields) => fields.borrow().iter().map(object).collect(),
            ReferenceType::Array(_, field_type, content) => {
                if let FieldType::Primitive(PrimitiveType::Char) = field_type {
                    let bytes: Vec<u16> = content.borrow().iter().map(|e| if let Value::Integer(val) = e {*val as u16} else {0}).collect();
                    vec![String::from_utf16(bytes.as_slice()).unwrap()]
                } else {
                    content.borrow().iter().map(object).collect()
                }
            }
        }
    }
}

impl Debug for ReferenceValue<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VRef")
            .field("object_id", &self.id)
            .field("class_id", &self.class_id)
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