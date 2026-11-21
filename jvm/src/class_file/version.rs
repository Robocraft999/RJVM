use strum_macros::FromRepr;

#[derive(Debug, Clone, PartialEq, Eq, FromRepr)]
#[repr(u16)]
pub enum ClassFileVersion{
    Jdk5  = 49,
    Jdk7  = 51,
    Jdk8  = 52,
    Jdk17 = 61,
    Jdk21 = 65,
    Jdk22 = 66,
}