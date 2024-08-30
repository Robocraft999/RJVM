use strum_macros::FromRepr;

#[derive(Debug, Clone, PartialEq, FromRepr)]
#[repr(u16)]
pub enum ClassFileVersion{
    Jdk7  = 51,
    Jdk17 = 61,
    Jdk21 = 65,
    Jdk22 = 66,
}