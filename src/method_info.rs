use crate::attribute::{Attribute, Code};
use crate::access_flags::MethodFlags;

#[derive(Debug)]
pub struct MethodInfo{
    pub flags: MethodFlags,
    pub name: String,
    pub descriptor: String,
    pub deprecated: bool,
    pub code: Option<Code>,
    pub attributes: Vec<Attribute>
}