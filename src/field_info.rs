use crate::attribute::{Attribute, ConstantValue};
use crate::access_flags::FieldFlags;

#[derive(Debug)]
pub struct FieldInfo{
    pub flags: FieldFlags,
    pub name: String,
    pub descriptor: String,
    pub deprecated: bool,
    pub constant_value: Option<ConstantValue>,
    pub attributes: Vec<Attribute>
}