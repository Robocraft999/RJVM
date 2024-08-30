use strum_macros::FromRepr;

#[derive(Debug, PartialEq, FromRepr)]
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

pub type ClassFlags = Vec<ClassFlag>;

pub fn parse_class_flags(flags: u16) -> ClassFlags{
    let mut flags_parsed = Vec::new();
    for i in 0..16{
        if let Some(flag) = ClassFlag::from_repr(1 << i){
            if flags & (1 << i) != 0{
                flags_parsed.push(flag)
            }
        }
    }
    flags_parsed
}

#[derive(Debug, PartialEq, FromRepr)]
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

#[derive(Debug, PartialEq, FromRepr)]
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

