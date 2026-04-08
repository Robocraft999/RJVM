use strum_macros::FromRepr;

#[derive(Debug, PartialEq, FromRepr, Clone)]
#[repr(u16)]
pub enum ClassFlag{
    Public     = 0x0001,
    Final      = 0x0010,
    Super      = 0x0020,
    Interface  = 0x0200,
    Abstract   = 0x0400,
    Synthetic  = 0x1000,
    Annotation = 0x2000,
    Enum       = 0x4000,
    Module     = 0x8000,
}

#[derive(Debug, PartialEq, FromRepr, Clone)]
#[repr(u16)]
pub enum FieldFlag{
    Public     = 0x0001,
    Private    = 0x0002,
    Protected  = 0x0004,
    Static     = 0x0008,
    Final      = 0x0010,
    Volatile   = 0x0040,
    Transient  = 0x0080,
    Synthetic  = 0x1000,
    Enum       = 0x4000,
}

pub type FieldFlags = Vec<FieldFlag>;

pub fn parse_field_flags(flags: u16) -> FieldFlags{
    let mut flags_parsed = Vec::new();
    for i in 0..16{
        if let Some(flag) = FieldFlag::from_repr(1 << i){
            if flags & (1 << i) != 0{
                flags_parsed.push(flag)
            }
        }
    }
    flags_parsed
}

#[derive(Debug, PartialEq, FromRepr, Clone)]
#[repr(u16)]
pub enum MethodFlag{
    Public       = 0x0001,
    Private      = 0x0002,
    Protected    = 0x0004,
    Static       = 0x0008,
    Final        = 0x0010,
    Synchronized = 0x0020,
    Bridge       = 0x0040,
    VarArgs      = 0x0080,
    Native       = 0x0100,
    Abstract     = 0x0400,
    Strict       = 0x0800,
    Synthetic    = 0x1000,
}

pub type MethodFlags = Vec<MethodFlag>;

pub fn parse_method_flags(flags: u16) -> MethodFlags{
    let mut flags_parsed = Vec::new();
    for i in 0..16{
        if let Some(flag) = MethodFlag::from_repr(1 << i){
            if flags & (1 << i) != 0{
                flags_parsed.push(flag)
            }
        }
    }
    flags_parsed
}

mod tests{
    use crate::access_flags::{parse_method_flags, MethodFlag};

    #[test]
    fn test_parse_method_flags(){
        assert_eq!(parse_method_flags(0x1), vec!(MethodFlag::Public));
        assert_eq!(parse_method_flags(0x2), vec!(MethodFlag::Private));
        assert_eq!(parse_method_flags(0x4), vec!(MethodFlag::Protected));
        assert_eq!(parse_method_flags(0x8), vec!(MethodFlag::Static));
        assert_eq!(parse_method_flags(0x10), vec!(MethodFlag::Final));
        assert_eq!(parse_method_flags(0x20), vec!(MethodFlag::Synchronized));
        assert_eq!(parse_method_flags(0x40), vec!(MethodFlag::Bridge));
        assert_eq!(parse_method_flags(0x80), vec!(MethodFlag::VarArgs));
        assert_eq!(parse_method_flags(0x100), vec!(MethodFlag::Native));
        assert_eq!(parse_method_flags(0x400), vec!(MethodFlag::Abstract));
        assert_eq!(parse_method_flags(0x800), vec!(MethodFlag::Strict));
        assert_eq!(parse_method_flags(0x1000), vec!(MethodFlag::Synthetic));

        assert_eq!(parse_method_flags(0x10A), vec![MethodFlag::Private, MethodFlag::Static, MethodFlag::Native]);
    }
}

