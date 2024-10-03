use std::fmt::{Debug, Formatter};

use crate::bytecode::{Instruction, printable_instructions};
use crate::bytes::{parse_u1, parse_u2};
use crate::class_file::get_constant_printable;
use crate::constants::ConstantPool;
use crate::error::ClassParseError;
use crate::vm::VmError;

#[derive(Debug, Clone)]
pub struct Attribute{
    pub name: String,
    pub info: Vec<u8>
}

#[derive(Debug, Clone, Copy)]
pub struct ConstantValue{
    pub constant_index: u16
}

#[derive(Clone)]
pub struct Code{
    pub max_stack: u16,
    pub max_locals: u16,
    //TODO add proper struct
    pub code: Vec<u8>,
    //TODO add remaining fields (https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.7.3)
    pub attributes: Vec<Attribute>,
    pub line_number_table: Option<LineNumberTable>,
}

impl Debug for Code{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeAttribute")
            .field("max_stack", &self.max_stack)
            .field("max_locals", &self.max_locals)
            .field("bytecode", &format_args!("{:?}", printable_instructions(&self.code)))
            .field("line_number_table", &format_args!("{:#?}", self.line_number_table))
            .field("attributes", &format_args!("{:#?}", self.attributes))
            .finish()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LineNumberTable(pub Vec<LineNumberTableEntry>);

#[derive(PartialEq, Clone)]
pub struct LineNumberTableEntry{
    pub program_counter: ProgramCounter,
    pub line_number: LineNumber
}

impl Debug for LineNumberTableEntry{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LineNumberTableEntry[{:?}, {:?}]", self.program_counter, self.line_number)
    }
}

impl LineNumberTableEntry{
    pub fn new(program_counter: ProgramCounter, line_number: LineNumber) -> Self{
        Self{
            program_counter,
            line_number,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProgramCounter(pub u16);
#[derive(Debug, PartialEq, Clone)]
pub struct LineNumber(pub u16);

#[derive(Clone, Debug, PartialEq)]
pub struct VisibleRuntimeAnnotations(pub Vec<Annotation>);

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation{
    pub name: String,
    pub values: Vec<ElementValuePair>,
}

impl Annotation {
    pub fn new<I: Iterator<Item=u8>>(constant_pool: &ConstantPool, bytes: &mut I) -> Result<Self, ClassParseError> {
        let name = get_constant_printable(&constant_pool, parse_u2(bytes)?);
        let num_element_value_pairs = parse_u2(bytes)?;
        let mut values = Vec::with_capacity(num_element_value_pairs as usize);
        for _ in 0..num_element_value_pairs {
            let element_name = get_constant_printable(&constant_pool, parse_u2(bytes)?);
            let value = Self::parse_element_value(constant_pool, bytes)?;
            let value_pair = ElementValuePair(element_name, value);
            values.push(value_pair);
        }
        Ok(Annotation{
            name,
            values,
        })
    }

    fn parse_element_value<I: Iterator<Item=u8>>(constant_pool: &ConstantPool, bytes: &mut I) -> Result<ElementValue, ClassParseError>{
        let tag = parse_u1(bytes)? as char;
        Ok(match tag {
            'B' | 'C' | 'S' | 'I' | 'J' | 'F' | 'D' | 'Z' => ElementValue::Primitive(parse_u2(bytes)?),
            's' => ElementValue::String(get_constant_printable(&constant_pool, parse_u2(bytes)?)),
            'c' => ElementValue::Class(get_constant_printable(&constant_pool, parse_u2(bytes)?)),
            'e' => ElementValue::Enum(get_constant_printable(&constant_pool, parse_u2(bytes)?), get_constant_printable(&constant_pool, parse_u2(bytes)?)),
            '[' => {
                //FIXME breaks when loading java native annotations which cross reference and create a loop
                let num_elements = parse_u2(bytes)?;
                let values = (0..num_elements).map(|_| Self::parse_element_value(constant_pool, bytes)).collect::<Result<Vec<_>, _>>()?;
                ElementValue::Array(values)
            }
            _ => unimplemented!(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementValuePair(pub String, pub ElementValue);

#[derive(Clone, Debug, PartialEq)]
pub enum ElementValue{
    Primitive(u16),
    String(String),
    Enum(String, String),
    Class(String),
    Annotation(Annotation),
    Array(Vec<ElementValue>),
}

