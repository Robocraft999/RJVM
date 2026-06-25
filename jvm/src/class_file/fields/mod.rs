use crate::access_flags::FieldFlag;
use crate::class_file::fields::attributes::FieldInfoAttributes;
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;

pub mod attributes;
pub mod field_type;

#[derive(Debug)]
pub struct FieldInfo{
    pub flags: u16,
    pub name: String,
    pub field_type: FieldType,
    pub slot: usize,
    pub holder_id: ClassId,
    pub attributes: FieldInfoAttributes,
}

impl FieldInfo {
    pub fn is_static(&self) -> bool {
        self.flags & FieldFlag::Static as u16 > 0
    }
}

pub fn primitive_to_wrapper_name(prim_name: &str) -> String{
    match prim_name {
        "boolean" => "java/lang/Boolean",
        "byte" => "java/lang/Byte",
        "char" => "java/lang/Character",
        "short" => "java/lang/Short",
        "int" => "java/lang/Integer",
        "long" => "java/lang/Long",
        "float" => "java/lang/Float",
        "double" => "java/lang/Double",
        "void" => "java/lang/Void",
        _ => unreachable!("Type is not primitive")
    }.to_string()
}

pub fn primitive_type_to_class_name(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::Integer => "int".to_string(),
        PrimitiveType::Long    => "long".to_string(),
        PrimitiveType::Short   => "short".to_string(),
        PrimitiveType::Char    => "char".to_string(),
        PrimitiveType::Byte    => "byte".to_string(),
        PrimitiveType::Float   => "float".to_string(),
        PrimitiveType::Double  => "double".to_string(),
        PrimitiveType::Boolean => "boolean".to_string(),
    }
}

pub fn primitive_type_to_descriptor(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::Boolean => String::from("Z"),
        PrimitiveType::Byte => String::from("B"),
        PrimitiveType::Char => String::from("C"),
        PrimitiveType::Double => String::from("D"),
        PrimitiveType::Float => String::from("F"),
        PrimitiveType::Integer => String::from("I"),
        PrimitiveType::Long => String::from("J"),
        PrimitiveType::Short => String::from("S"),
    }
}

/**
* class_name without L{class_name};
*/
pub fn get_class_descriptor(class_name: &str) -> String{
    match class_name {
        // primitive classes
        "boolean" => "Z".to_string(),
        "byte" => "B".to_string(),
        "char" => "C".to_string(),
        "short" => "S".to_string(),
        "int" => "I".to_string(),
        "long" => "J".to_string(),
        "float" => "F".to_string(),
        "double" => "D".to_string(),
        "void" => "V".to_string(),
        // already class / array
        _ => {
            if class_name.starts_with("[") {
                class_name.to_string()
            } else {
                "L".to_string() + class_name + ";"
            }
        }
    }
}