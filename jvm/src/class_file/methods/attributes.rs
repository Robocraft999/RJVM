use crate::class_file::attributes::{gen_nom_unfucker, Deprecated, ElementValue, RuntimeAnnotations, Signature, Synthetic};
use crate::class_file::nom::RawAttribute;
use crate::vm::result::VMResult;
use crate::vm::VmError;
use nom::number::{be_u16, be_u32};
use nom_derive::{NomBE, Parse};

#[derive(Debug, Clone, Default)]
pub struct MethodInfoAttributes{
    pub code: Option<Code>,
    pub exceptions: Option<Exceptions>,
    pub synthetic: Option<Synthetic>,
    pub signature: Option<Signature>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeAnnotations>,
    pub runtime_invisible_annotations: Vec<RuntimeAnnotations>,
    pub runtime_visible_parameter_annotations: Vec<Vec<u8>>,
    pub runtime_invisible_parameter_annotations: Vec<Vec<u8>>,
    pub runtime_visible_type_annotations: Vec<Vec<u8>>,
    pub runtime_invisible_type_annotations: Vec<Vec<u8>>,
    pub annotation_default: Option<AnnotationDefault>,
    pub method_parameters: Option<MethodParameters>
}

#[derive(Debug, Clone, Default)]
pub struct CodeAttributes {
    pub stack_map_table: Option<StackMapTable>,
    pub line_number_tables: Vec<LineNumberTable>,
    pub local_variable_tables: Vec<LocalVariableTable>,
    pub local_variable_type_tables: Vec<Vec<u8>>,
}

gen_nom_unfucker!(Code, parse_code);
gen_nom_unfucker!(Exceptions, parse_exceptions);
gen_nom_unfucker!(Synthetic, parse_synthetic);
gen_nom_unfucker!(Signature, parse_signature);
gen_nom_unfucker!(Deprecated, parse_deprecated);
gen_nom_unfucker!(RuntimeAnnotations, parse_runtime_visible_annotations);
gen_nom_unfucker!(AnnotationDefault, parse_annotation_default_default);

gen_nom_unfucker!(LineNumberTable, parse_line_number_table);
gen_nom_unfucker!(StackMapTable, parse_stack_map_table);
gen_nom_unfucker!(LocalVariableTable, parse_local_variable_table);

impl MethodInfoAttributes {
    pub fn set(&mut self, name: &str, bytes: Vec<u8>) -> VMResult<()>{
        Ok(match name{
            "Code" => self.code = Some(parse_code(&bytes)?),
            "Exceptions" => self.exceptions = Some(parse_exceptions(&bytes)?),
            "Synthetic" => self.synthetic = Some(parse_synthetic(&bytes)?),
            "Signature" => self.signature = Some(parse_signature(&bytes)?),
            "Deprecated" => self.deprecated = Some(parse_deprecated(&bytes)?),
            "RuntimeVisibleAnnotations" => self.runtime_visible_annotations.push(parse_runtime_visible_annotations(&bytes)?),
            "AnnotationDefault" => self.annotation_default = Some(parse_annotation_default_default(&bytes)?),
            invalid => return Err(VmError::ValidationError(format!("Unknown class file attribute: {}", invalid))),
        })
    }
}

impl CodeAttributes {
    pub fn set(&mut self, name: &str, bytes: Vec<u8>) -> VMResult<()>{
        Ok(match name{
            "LineNumberTable" => self.line_number_tables.push(parse_line_number_table(&bytes)?),
            "StackMapTable" => (),/*self.stack_map_table = Some(parse_stack_map_table(&bytes)?),*/
            "LocalVariableTable" => self.local_variable_tables.push(parse_local_variable_table(&bytes)?),
            "LocalVariableTypeTable" => (),
            invalid => return Err(VmError::ValidationError(format!("Unknown class file attribute: {}", invalid))),
        })
    }
}

#[derive(Debug, Clone, NomBE)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    #[nom(LengthCount = "be_u32()")]
    pub code: Vec<u8>,
    #[nom(LengthCount = "be_u16()")]
    pub exception_table: Vec<ExceptionTableEntry>,
    #[nom(LengthCount = "be_u16()")]
    pub raw_attributes: Vec<RawAttribute>,
    #[nom(Ignore)]
    pub attributes: CodeAttributes,
}

#[derive(Debug, Clone, NomBE)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct StackMapTable{

}

#[derive(Debug, Clone, NomBE)]
pub struct LineNumberTable {
    #[nom(LengthCount = "be_u16()")]
    pub line_number_table: Vec<LineNumberTableEntry>
}

#[derive(Debug, Clone, NomBE)]
pub struct LineNumberTableEntry {
    pub start_pc: u16,
    pub line_number: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct LocalVariableTable{
    #[nom(LengthCount = "be_u16()")]
    pub local_variable_table: Vec<LocalVariableTableEntry>
}

#[derive(Debug, Clone, NomBE)]
pub struct LocalVariableTableEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub index: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct Exceptions {
    #[nom(LengthCount = "be_u16()")]
    pub exception_index_table: Vec<u16>,
}

#[derive(Debug, Clone, NomBE)]
pub struct AnnotationDefault{
    pub default_value: ElementValue
}

#[derive(Debug, Clone, NomBE)]
pub struct MethodParameters {
    #[nom(LengthCount = "be_u16()")]
    pub parameters: Vec<MethodParameter>,
}

#[derive(Debug, Clone, NomBE)]
pub struct MethodParameter {
    pub name_index: u16,
    pub access_flags: u16,
}