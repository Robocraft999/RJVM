use std::fmt::{Debug, Formatter};

use crate::bytecode::Instruction;

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
    pub code: Vec<Instruction>,
    //TODO add remaining fields (https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.7.3)
    pub attributes: Vec<Attribute>,
    pub line_number_table: Option<LineNumberTable>,
}

impl Debug for Code{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeAttribute")
            .field("max_stack", &self.max_stack)
            .field("max_locals", &self.max_locals)
            .field("bytecode", &format_args!("{:02x?}", self.code))
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