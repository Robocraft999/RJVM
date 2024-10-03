use regex::Regex;

use crate::access_flags::{MethodFlag, MethodFlags};
use crate::attribute::{Attribute, Code};
use crate::field_info::{FieldType, parse_field_type, PrimitiveType};

#[derive(Debug)]
pub struct MethodInfo{
    pub flags: MethodFlags,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub deprecated: bool,
    pub code: Option<Code>,
    pub attributes: Vec<Attribute>
}

impl MethodInfo{
    pub fn get_args_count(&self) -> usize{
        self.descriptor.args.len()
    }

    pub fn is_native(&self) -> bool {
        self.flags.contains(&MethodFlag::Native)
    }

    pub fn is_static(&self) -> bool{
        self.flags.contains(&MethodFlag::Static)
    }

    pub fn is_abstract(&self) -> bool { self.flags.contains(&MethodFlag::Abstract) }
}

#[derive(Debug)]
pub struct MethodDescriptor{
    raw: String,
    pub args: Vec<FieldType>,
    pub return_type: Option<FieldType>,
}

impl MethodDescriptor{
    pub fn new(raw_string: String) -> Self{
        let r = Regex::new(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9]+);|(?<void>V))").unwrap();
        let mut args = Vec::new();
        let mut void_return = false;
        for cap in r.captures_iter(raw_string.as_str()){
            if cap.name("void").is_some() {
                void_return = true;
                continue
            }

            /*let primitive = cap.name("primitive");
            let object = cap.name("object");

            let field_type = if let Some(prim) = primitive{
                FieldType::Primitive(PrimitiveType::from_str(prim.as_str()).unwrap())
            } else if let Some(obj) = object{
                FieldType::Object(String::from(obj.as_str()))
            } else {
                unreachable!("Type {cap:?} is neither object nor primitive")
            };

            args.push(if let Some(array) = cap.name("array"){
                let dims = array.len();
                FieldType::Array(dims, Box::new(field_type))
            } else {
                field_type
            });*/
            args.push(parse_field_type(cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), cap.name("array").map(|m| m.as_str())))
        }

        let return_type = if void_return {None} else {args.pop()};

        Self{
            raw: raw_string,
            args,
            return_type,
        }
    }

    pub fn matches(&self, other: &str) -> bool{
        //TODO maybe parse other or do it better in some way
        self.raw == other
    }

    pub fn as_str(&self) -> &str{
        self.raw.as_str()
    }
}

impl PartialEq for MethodDescriptor{
    fn eq(&self, other: &Self) -> bool {
        self.matches(other.raw.as_str())
    }
}