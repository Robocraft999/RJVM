use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;

use crate::access_flags::{MethodFlag, MethodFlags};
use crate::attribute::{Attribute, Code, ExceptionTable, Exceptions, ProgramCounter};
use crate::field_info::FieldType;
use crate::vm::bytecode::InstructionBlock;
use crate::vm::VmError;

#[derive(Debug)]
pub struct MethodInfo{
    pub flags: MethodFlags,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub deprecated: bool,
    pub code: Option<Code>,
    pub code_blocks: Option<BTreeMap<u16, InstructionBlock>>,
    pub exceptions: Option<Exceptions>,
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
    
    pub fn has_exception_handler(&self) -> bool {
        if let Some(code) = &self.code {
            code.exception_table.0.len() > 0
        } else {
            false
        }
    }
    
    pub fn get_exception_handlers(&self) -> Option<&ExceptionTable> {
        if let Some(code) = &self.code {
            Some(&code.exception_table)
        } else {
            None
        }
    }

    pub fn get_code_block_at(&self, pc: ProgramCounter) -> &InstructionBlock{
        &self.code_blocks.as_ref().unwrap().get(&pc.0).ok_or(VmError::ValidationError(format!("Code block out of bounds: {}, {:?}", pc.0, self.code_blocks))).unwrap()
    }

    pub fn next_pc(&self, pc: ProgramCounter) -> Option<u16>{
        self.code_blocks.as_ref().map(|blocks| blocks.range(pc.0+1..).next()).flatten().map(|t|*t.0)
    }

    pub fn previous_pc(&self, pc: ProgramCounter) -> u16{
        self.code_blocks.as_ref().map(|blocks| blocks.range(..pc.0).next_back()).flatten().map(|t|*t.0).unwrap_or(0)
    }
}

impl PartialEq for MethodInfo{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.descriptor == other.descriptor
    }
}

impl Eq for MethodInfo{}

#[derive(Debug)]
pub struct MethodDescriptor{
    raw: String,
    pub args: Vec<FieldType>,
    pub return_type: Option<FieldType>,
}

impl Hash for MethodDescriptor{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

static PATTERN: Lazy<Regex> = lazy_regex!(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[\/a-zA-Z$0-9]+);|(?<void>V))");

impl MethodDescriptor{
    pub fn new(raw_string: String) -> Self{
        let mut args = Vec::new();
        let mut void_return = false;
        for cap in PATTERN.captures_iter(raw_string.as_str()){
            if cap.name("void").is_some() {
                void_return = true;
                continue
            }
            let object = cap.name("object").map(|m| m.as_str());
            let primitive = cap.name("primitive").map(|m| m.as_str());
            let array = cap.name("array").map(|m| m.as_str());
            //FIXME error handling
            args.push(FieldType::from_raw_parts(object, primitive, array).unwrap());
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

impl Eq for MethodDescriptor{}