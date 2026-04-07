use crate::class_file::attributes::{gen_nom_unfucker, Deprecated, RuntimeAnnotations, Signature, Synthetic};
use crate::vm::result::VMResult;
use crate::vm::VmError;
use nom_derive::{NomBE, Parse};

#[derive(Debug, Clone, Default)]
pub struct FieldInfoAttributes{
    pub constant_value: Option<ConstantValue>,
    pub synthetic: Option<Synthetic>,
    pub signature: Option<Signature>,
    pub deprecated: Option<Deprecated>,
    pub runtime_visible_annotations: Vec<RuntimeAnnotations>,
    pub runtime_invisible_annotations: Vec<RuntimeAnnotations>,
    pub runtime_visible_type_annotations: Vec<Vec<u8>>,
    pub runtime_invisible_type_annotations: Vec<Vec<u8>>,
}

gen_nom_unfucker!(ConstantValue, parse_constant_value);
gen_nom_unfucker!(Synthetic, parse_synthetic);
gen_nom_unfucker!(Signature, parse_signature);
gen_nom_unfucker!(Deprecated, parse_deprecated);
gen_nom_unfucker!(RuntimeAnnotations, parse_runtime_visible_annotations);

impl FieldInfoAttributes {
    pub fn set(&mut self, name: &str, bytes: Vec<u8>) -> VMResult<()>{
        Ok(match name{
            "ConstantValue" => self.constant_value = Some(parse_constant_value(&bytes)?),
            "Synthetic" => self.synthetic = Some(parse_synthetic(&bytes)?),
            "Signature" => self.signature = Some(parse_signature(&bytes)?),
            "Deprecated" => self.deprecated = Some(parse_deprecated(&bytes)?),
            "RuntimeVisibleAnnotations" => self.runtime_visible_annotations.push(parse_runtime_visible_annotations(&bytes)?),
            invalid => return Err(VmError::ValidationError(format!("Unknown class file attribute: {}", invalid))),
        })
    }
}

#[derive(Debug, Clone, NomBE)]
pub struct ConstantValue {
    pub constantvalue_index: u16,
}