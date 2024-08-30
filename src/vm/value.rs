use std::cell::RefCell;
use std::fmt::{Debug, Formatter};

use crate::vm::class::ClassId;

#[derive(Debug, PartialEq, Default, Clone)]
pub enum Value<'a>{
    #[default]
    Uninitialized,
    Object(ObjectRef<'a>),


    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),

    Null,
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