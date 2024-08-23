use std::fmt::Debug;
use std::fmt::Formatter;
use Constant::*;

pub enum Constant{
    Class = 7,
    Fieldref = 9,
    Methodref = 10,
    InterfaceMethodref = 11,
    String = 8,
    Integer = 3,
    Float = 4,
    Long = 5,
    Double = 6,
    NameAndType = 12,
    Utf8 = 1,
    MethodHandle = 15,
    MethodType = 16,
    InvokeDynamic = 18
}

impl Debug for Constant{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> { 
        let string = match self {
            Class => "CONSTANT_Class",
            Fieldref => "CONSTANT_Fieldref",
            Methodref => "CONSTANT_Methodref",
            InterfaceMethodref => "CONSTANT_InterfaceMethodref",
            String => "CONSTANT_String",
            Integer => "CONSTANT_Integer",
            Float => "CONSTANT_Float",
            Long => "CONSTANT_Long",
            Double => "CONSTANT_Double",
            NameAndType => "CONSTANT_NameAndType",
            Utf8 => "CONSTANT_Utf8",
            MethodHandle => "CONSTANT_MethodHandle",
            MethodType => "CONSTANT_MethodType",
            InvokeDynamic => "CONSTANT_InvokeDynamic"
        };
        write!(f, "{string}")
    }
}

impl From<u8> for Constant{
    fn from(byte: u8) -> Self { 
        match byte {
            1 => Utf8,
            3 => Integer,
            4 => Float,
            5 => Long,
            6 => Double,
            7 => Class,
            8 => String,
            9 => Fieldref,
            10 => Methodref,
            11 => InterfaceMethodref,
            12 => NameAndType,
            15 => MethodHandle,
            16 => MethodType,
            18 => InvokeDynamic,
            _ => panic!("There is no Constant of tag {}", byte)
        }
    }
}

pub const ACC_PUBLIC: u16     = 0x0001; //000000000000001
pub const ACC_FINAL: u16      = 0x0010; //000000000010000
pub const ACC_SUPER: u16      = 0x0020; //000000000100000
pub const ACC_INTERFACE: u16  = 0x0200; //000001000000000
pub const ACC_ABSTRACT: u16   = 0x0400; //000010000000000
pub const ACC_SYNTHETIC: u16  = 0x1000; //001000000000000
pub const ACC_ANNOTATION: u16 = 0x2000; //010000000000000
pub const ACC_ENUM: u16       = 0x4000; //100000000000000