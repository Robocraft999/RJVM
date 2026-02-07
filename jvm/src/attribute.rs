use std::fmt::{Debug, Formatter};

use crate::bytecode::printable_instructions;
use crate::bytes::{parse_u1, parse_u2};
use crate::class_file::get_constant_printable;
use crate::constants::{BytecodeBehavior, ConstantPool};
use crate::error::ClassParseError;

#[derive(Debug, Clone)]
pub struct Attribute{
    pub name: String,
    pub info: Vec<u8>
}

#[derive(Debug, Clone, Default)]
pub struct ClassFileAttributes{
    pub inner_classes: Option<InnerClasses>,
    pub enclosing_method: Option<EnclosingMethod>,
    pub synthetic: Option<Attribute>,
    pub signature: Option<Attribute>,
    pub source_file: Option<SourceFile>,
    pub source_debug_extension: Option<Attribute>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeVisibleAnnotations>,
    pub runtime_invisible_annotations: Vec<Attribute>,
    pub runtime_visible_type_annotations: Vec<Attribute>,
    pub runtime_invisible_type_annotations: Vec<Attribute>,
    pub bootstrap_methods: Option<BootstrapMethods>,
}

pub struct FieldInfoAttributes{
    pub constant_value: Option<ConstantValue>,
    pub synthetic: Option<Attribute>,
    pub signature: Option<Attribute>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeVisibleAnnotations>,
    pub runtime_invisible_annotations: Vec<Attribute>,
    pub runtime_visible_type_annotations: Vec<Attribute>,
    pub runtime_invisible_type_annotations: Vec<Attribute>,
}

pub struct MethodInfoAttributes{
    pub code: Option<Code>,
    pub exceptions: Option<Exceptions>,
    pub synthetic: Option<Attribute>,
    pub signature: Option<Attribute>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeVisibleAnnotations>,
    pub runtime_invisible_annotations: Vec<Attribute>,
    pub runtime_visible_parameter_annotations: Vec<Attribute>,
    pub runtime_invisible_parameter_annotations: Vec<Attribute>,
    pub runtime_visible_type_annotations: Vec<Attribute>,
    pub runtime_invisible_type_annotations: Vec<Attribute>,
    pub annotation_default: Option<Attribute>,
    pub method_parameters: Option<Attribute>
}

pub struct CodeAttributes{
    pub stack_map_table: Option<Attribute>,
    pub line_number_tables: Vec<LineNumberTable>,
    pub local_variable_tables: Vec<Attribute>,
    pub local_variable_type_tables: Vec<Attribute>,
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
    //TODO add remaining fields (https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.7.3)
    pub attributes: Vec<Attribute>,
    pub line_number_table: Option<LineNumberTable>,
    pub exception_table: ExceptionTable,
}

impl Debug for Code{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeAttribute")
            .field("max_stack", &self.max_stack)
            .field("max_locals", &self.max_locals)
            .field("bytecode", &format_args!("{:?}", printable_instructions(&self.code)))
            .field("line_number_table", &format_args!("{:#?}", self.line_number_table))
            .field("exception_table", &format_args!("{:#?}", self.exception_table))
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
pub struct ExceptionTable(pub Vec<ExceptionTableEntry>);

#[derive(Debug, PartialEq, Clone)]
pub struct ExceptionTableEntry{
    pub start_pc: ProgramCounter,
    pub end_pc: ProgramCounter,
    pub handler_pc: ProgramCounter,
    pub catch_type: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ProgramCounter(pub u16);
#[derive(Debug, PartialEq, Clone)]
pub struct LineNumber(pub u16);

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeVisibleAnnotations(pub Vec<Annotation>);

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

#[derive(Clone, Debug, PartialEq)]
pub struct BootstrapMethods(pub Vec<BootstrapMethod>);

#[derive(Clone, Debug, PartialEq)]
pub struct BootstrapMethod{
    pub bootstrap_method_ref_index: u16,
    pub arguments_indices: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exceptions(pub Vec<String>);

#[derive(Clone, Debug, PartialEq)]
pub struct EnclosingMethod{
    pub class_index: u16,
    pub method_index: u16,
}

pub type InnerClasses = Vec<InnerClass>;

#[derive(Clone, Debug, PartialEq)]
pub struct InnerClass{
    pub inner_class_info_index: u16,
    pub outer_class_info_index: u16,
    pub inner_name_index: u16,
    pub inner_class_access_flags: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct Deprecated;