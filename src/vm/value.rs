use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use crate::field_info::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;

#[derive(PartialEq, Default, Clone)]
pub enum Value<'a>{
    #[default]
    Uninitialized,
    Object(ObjectRef<'a>),
    Array(FieldType, ArrayRef<'a>),

    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),

    Null,
}

impl Debug for Value<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Array(FieldType::Array(size, field_type), array) => {
                if *field_type.as_ref() == FieldType::Primitive(PrimitiveType::Char){
                    write!(f, "String[{:?}]", String::from_utf8(array.borrow().iter().map(|e| if let Value::Integer(val) = e {*val as u8} else {0}).collect()))
                } else {
                    write!(f ,"{:?}[{}] = {:?}", field_type, size, &array.borrow())
                }
            }
            Value::Array(FieldType::Primitive(_), _) => {write!(f,"???")}
            Value::Array(FieldType::Object(_), _) => {write!(f,"!!!")}
            Value::Uninitialized => write!(f, "VUninitialized"),
            Value::Null => write!(f, "VNull"),
            Value::Object(object) => write!(f, "VObject({:?})", object),
            Value::Integer(value) => write!(f, "VInt ({})", value),
            Value::Long(value) => write!(f, "VLong ({})", value),
            Value::Float(value) => write!(f, "VFloat ({})", value),
            Value::Double(value) => write!(f, "VDouble ({})", value),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct ObjectValue<'a>{
    pub(crate) id: ClassId,
    pub(crate) fields: RefCell<Vec<Value<'a>>>
}

impl<'a> ObjectValue<'a>{
    pub fn set_field(&self, index: usize, value: Value<'a>) {
        self.fields.borrow_mut()[index] = value
    }

    pub fn get_field(&self, index: usize) -> Value<'a>{
        self.fields.borrow()[index].clone()
    }
}

impl Debug for ObjectValue<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Object")
            .field("id", &self.id)
            .field("fields", &format_args!("{:?}", self.fields.borrow()))
            .finish()
    }
}

pub type ObjectRef<'a> = &'a ObjectValue<'a>;
pub type ArrayRef<'a> = Rc<RefCell<Vec<Value<'a>>>>;