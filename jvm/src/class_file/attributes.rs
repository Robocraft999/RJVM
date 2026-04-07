use crate::vm::result::VMResult;
use crate::vm::VmError;
use nom::combinator::map;
use nom::error::Error;
use nom::multi::length_count;
use nom::number::{be_u16, be_u8};
use nom::IResult;
use nom_derive::{NomBE, Parse, Parser};

#[derive(Debug, Clone, Default)]
pub struct ClassFileAttributes{
    pub inner_classes: Option<InnerClasses>,
    pub enclosing_method: Option<EnclosingMethod>,
    pub synthetic: Option<Synthetic>,
    pub signature: Option<Signature>,
    pub source_file: Option<SourceFile>,
    pub source_debug_extension: Option<Vec<u8>>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeAnnotations>,
    pub runtime_invisible_annotations: Vec<RuntimeAnnotations>,
    // FIXME create the structs if you want a headache
    pub runtime_visible_type_annotations: Vec<Vec<u8>>,
    pub runtime_invisible_type_annotations: Vec<Vec<u8>>,
    pub bootstrap_methods: Option<BootstrapMethods>,
}

macro_rules! gen_nom_unfucker {
    ($typ:ty, $name:ident) => {
        fn $name(bytes: &[u8]) -> VMResult<$typ>{
            let (_, parsed) = <$typ>::parse(bytes)
        .map_err(|e| VmError::ValidationError(format!("{:?}", e)))?;
    Ok(parsed)
        }
    };
}

pub(crate) use gen_nom_unfucker;

gen_nom_unfucker!(InnerClasses, parse_inner_classes);
gen_nom_unfucker!(EnclosingMethod, parse_enclosing_method);
gen_nom_unfucker!(Synthetic, parse_synthetic);
gen_nom_unfucker!(Signature, parse_signature);
gen_nom_unfucker!(SourceFile, parse_source_file);
gen_nom_unfucker!(RuntimeAnnotations, parse_runtime_visible_annotations);
gen_nom_unfucker!(BootstrapMethods, parse_bootstrap_methods);

impl ClassFileAttributes {
    pub fn set(&mut self, name: &str, bytes: Vec<u8>) -> VMResult<()>{
        Ok(match name{
            "InnerClasses" => self.inner_classes = Some(parse_inner_classes(&bytes)?),
            "EnclosingMethod" => self.enclosing_method = Some(parse_enclosing_method(&bytes)?),
            "Synthetic" => self.synthetic = Some(parse_synthetic(&bytes)?),
            "Signature" => self.signature = Some(parse_signature(&bytes)?),
            "SourceFile" => self.source_file = Some(parse_source_file(&bytes)?),
            "RuntimeVisibleAnnotations" => self.runtime_visible_annotations.push(parse_runtime_visible_annotations(&bytes)?),
            "BootstrapMethods" => self.bootstrap_methods = Some(parse_bootstrap_methods(&bytes)?),
            invalid => return Err(VmError::ValidationError(format!("Unknown class file attribute: {}", invalid))),
        })
    }
}

#[derive(Debug, Clone, NomBE)]
pub struct InnerClasses {
    #[nom(LengthCount = "be_u16()")]
    pub classes: Vec<InnerClass>,
}

#[derive(Debug, Clone, NomBE)]
pub struct InnerClass {
    pub inner_class_info_index: u16,
    pub outer_class_info_index: u16,
    pub inner_name_index: u16,
    pub inner_class_access_flags: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct EnclosingMethod {
    pub class_index: u16,
    pub method_index: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct Synthetic;

#[derive(Debug, Clone, NomBE)]
pub struct Signature {
    pub signature_index: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct SourceFile {
    pub source_file_index: u16,
}

#[derive(Debug, Clone, NomBE)]
pub struct Deprecated;

#[derive(Debug, Clone, NomBE)]
pub struct RuntimeAnnotations {
    #[nom(LengthCount = "be_u16()")]
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, NomBE)]
pub struct Annotation {
    pub type_index: u16,
    #[nom(LengthCount = "be_u16()")]
    pub element_value_pairs: Vec<ElementValuePair>,
}

#[derive(Debug, Clone, NomBE)]
pub struct ElementValuePair{
    element_name_index: u16,
    value: ElementValue,
}

#[derive(Debug, Clone)]
pub enum ElementValue {
    Const { const_value_index: u16 },
    Enum { type_name_index: u16, const_name_index: u16 },
    Class { class_info_index: u16 },
    Annotation { annotation_value: Annotation },
    Array { array_value: Vec<ElementValue> },
}

impl Parse<&[u8]> for ElementValue {
    fn parse(i: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        Self::parse_be(i)
    }

    fn parse_be(i: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (i, tag) = be_u8().parse(i)?;
        match tag as char{
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' | 's' => map(be_u16(), |const_value_index| ElementValue::Const { const_value_index }).parse(i),
            'e' => map((be_u16(), be_u16()), |(type_name_index, const_name_index)| ElementValue::Enum{ type_name_index, const_name_index }).parse(i),
            'c' => map(be_u16(), |class_info_index| ElementValue::Class{ class_info_index }).parse(i),
            '@' => map(Annotation::parse, |annotation_value| ElementValue::Annotation{ annotation_value }).parse(i),
            '[' => map(length_count(be_u16(), ElementValue::parse), |array_value| ElementValue::Array{ array_value }).parse(i),
            _ => Err(nom::Err::Error(Error::new(i, nom::error::ErrorKind::Alt))),
        }
    }
}

#[derive(Debug, Clone, Default, NomBE)]
pub struct BootstrapMethods {
    #[nom(LengthCount = "be_u16()")]
    pub bootstrap_methods: Vec<BootstrapMethod>,
}

#[derive(Debug, Clone, Default, NomBE)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: u16,
    #[nom(LengthCount = "be_u16()")]
    pub bootstrap_arguments: Vec<u16>,
}
