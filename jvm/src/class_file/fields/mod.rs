use crate::access_flags::field_flags;
use crate::class_file::fields::attributes::FieldInfoAttributes;
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;

pub mod attributes;
pub mod field_type;

#[derive(Debug, Clone)]
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
        self.flags & field_flags::STATIC > 0
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
    }.to_owned()
}

pub fn primitive_type_to_class_name(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::Integer => "int".to_owned(),
        PrimitiveType::Long    => "long".to_owned(),
        PrimitiveType::Short   => "short".to_owned(),
        PrimitiveType::Char    => "char".to_owned(),
        PrimitiveType::Byte    => "byte".to_owned(),
        PrimitiveType::Float   => "float".to_owned(),
        PrimitiveType::Double  => "double".to_owned(),
        PrimitiveType::Boolean => "boolean".to_owned(),
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
        "boolean" => "Z".to_owned(),
        "byte" => "B".to_owned(),
        "char" => "C".to_owned(),
        "short" => "S".to_owned(),
        "int" => "I".to_owned(),
        "long" => "J".to_owned(),
        "float" => "F".to_owned(),
        "double" => "D".to_owned(),
        "void" => "V".to_owned(),
        // already class / array
        _ => {
            if class_name.starts_with("[") {
                class_name.to_owned()
            } else {
                "L".to_string() + class_name + ";"
            }
        }
    }
}