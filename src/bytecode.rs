use strum_macros::FromRepr;
use crate::attribute::ProgramCounter;
use crate::bytecode::Instruction::*;
use crate::error::ClassParseError;
use crate::vm::VmError;

pub fn printable_instructions(code_bytes: &Vec<u8>) -> Vec<Instruction>{
    let mut iter = code_bytes.iter();
    let mut instructions = Vec::new();
    //TODO add error handling
    let mut pc = 0;
    loop{
        if let Ok((instruction, new_pc)) = parse_instruction(code_bytes, pc){
            instructions.push(instruction);
            pc = new_pc
        } else {
            break;
        }
    }
    instructions
}

fn parse_u1(code_bytes: &Vec<u8>, pc: &mut usize) -> Result<u8, VmError>{
    let res = *code_bytes.get(*pc).ok_or(VmError::ParseError(ClassParseError::ReadError))?;
    *pc += 1;
    Ok(res)
}

fn parse_i1(code_bytes: &Vec<u8>, pc: &mut usize) -> Result<i8, VmError>{
    Ok(parse_u1(code_bytes, pc)? as i8)
}

fn parse_u2(code_bytes: &Vec<u8>, pc: &mut usize) -> Result<u16, VmError>{
    let res = u16::from_be_bytes([parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?]);
    Ok(res)
}

fn parse_offset(code_bytes: &Vec<u8>, pc: &mut usize) -> Result<u16, VmError>{
    let instruction_pc = *pc - 1;
    let offset = parse_u2(code_bytes, pc)? as i16;
    Ok(((instruction_pc as i16) + offset) as u16)
}

pub fn parse_instruction(code_bytes: &Vec<u8>, mut pc: usize) -> Result<(Instruction, usize), VmError>{
    let opcode = parse_u1(code_bytes, &mut pc)?;
    let result = if let Some(instruction) = Instruction::from_repr(opcode){
        match instruction{
            INVOKEVIRTUAL(_) => INVOKEVIRTUAL(parse_u2(code_bytes, &mut pc)?),
            INVOKESPECIAL(_) => INVOKESPECIAL(parse_u2(code_bytes, &mut pc)?),
            INVOKESTATIC(_) => INVOKESTATIC(parse_u2(code_bytes, &mut pc)?),
            INVOKEINTERFACE(_, _, _) => INVOKEINTERFACE(parse_u2(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?),
            GETSTATIC(_) => GETSTATIC(parse_u2(code_bytes, &mut pc)?),
            PUTSTATIC(_) => PUTSTATIC(parse_u2(code_bytes, &mut pc)?),
            IF_ACMPNE(_) => IF_ACMPNE(parse_offset(code_bytes, &mut pc)?),
            IF_ACMPEQ(_) => IF_ACMPEQ(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPLE(_) => IF_ICMPLE(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPGE(_) => IF_ICMPGE(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPLT(_) => IF_ICMPLT(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPNE(_) => IF_ICMPNE(parse_offset(code_bytes, &mut pc)?),
            IFNONNULL(_) => IFNONNULL(parse_offset(code_bytes, &mut pc)?),
            IFNULL(_) => IFNULL(parse_offset(code_bytes, &mut pc)?),
            IFEQ(_) => IFEQ(parse_offset(code_bytes, &mut pc)?),
            IFNE(_) => IFNE(parse_offset(code_bytes, &mut pc)?),
            IFGE(_) => IFGE(parse_offset(code_bytes, &mut pc)?),
            IFGT(_) => IFGT(parse_offset(code_bytes, &mut pc)?),
            IFLT(_) => IFLT(parse_offset(code_bytes, &mut pc)?),
            IFLE(_) => IFLE(parse_offset(code_bytes, &mut pc)?),
            GOTO(_) => GOTO(parse_offset(code_bytes, &mut pc)?),
            LDC(_) => LDC(parse_u1(code_bytes, &mut pc)?),
            LDCW(_) => LDCW(parse_u2(code_bytes, &mut pc)?),
            LDC2W(_) => LDC2W(parse_u2(code_bytes, &mut pc)?),
            BIPUSH(_) => BIPUSH(parse_u1(code_bytes, &mut pc)?),
            SIPUSH(_) => SIPUSH(parse_u2(code_bytes, &mut pc)?),
            GETFIELD(_) => GETFIELD(parse_u2(code_bytes, &mut pc)?),
            PUTFIELD(_) => PUTFIELD(parse_u2(code_bytes, &mut pc)?),
            ISTORE(_) => ISTORE(parse_u1(code_bytes, &mut pc)?),
            ILOAD(_) => ILOAD(parse_u1(code_bytes, &mut pc)?),
            ASTORE(_) => ASTORE(parse_u1(code_bytes, &mut pc)?),
            ALOAD(_) => ALOAD(parse_u1(code_bytes, &mut pc)?),
            NEW(_) => NEW(parse_u2(code_bytes, &mut pc)?),
            ANEWARRAY(_) => ANEWARRAY(parse_u2(code_bytes, &mut pc)?),
            NEWARRAY(_) => NEWARRAY(parse_u1(code_bytes, &mut pc)?),
            IINC(_, _) => IINC(parse_u1(code_bytes, &mut pc)?, parse_i1(code_bytes, &mut pc)?),
            CHECKCAST(_) => CHECKCAST(parse_u2(code_bytes, &mut pc)?),
            RETURN | IRETURN | ARETURN | DRETURN | LRETURN | FRETURN |
            ALOAD0 | ALOAD1 | ALOAD2 | ALOAD3 | IALOAD | AALOAD |
            LLOAD0 | LLOAD1 | LLOAD2 |
            ILOAD0 | ILOAD1 | ILOAD2 | ILOAD3 |
            FLOAD0 | FLOAD1 | FLOAD2 |
            DLOAD0 |
            ACONST_NULL |
            ICONST0 | ICONST1 | ICONST2 | ICONST3 | ICONST4 | ICONST5 |
            LCONST0 | LCONST1 |
            FCONST0 | FCONST1 |
            ISTORE0 | ISTORE1 | ISTORE2 | ISTORE3 |
            ASTORE0 | ASTORE1 | ASTORE2 | ASTORE3 | IASTORE | AASTORE | CASTORE |
            LSTORE0 | LSTORE1 | LSTORE2 |
            DUP | LCMP | ATHROW | LADD | IADD | ISUB | IMUL | FMUL | ARRAYLENGTH | POP | NOP |
            LUSHR | ISHL | ISHR | IUSHR | IOR | IXOR | IAND | LAND |
            MONITORENTER | MONITOREXIT |
            D2I | L2I | I2F | F2I | I2L |
            FCMPG | FCMPL => instruction,
            _ => unreachable!("Instruction {:?} not initializable", instruction)
        }
    } else {
        unimplemented!("Instruction '0x{:x}' not supported yet", opcode);
    };
    Ok((result, pc))
}

#[derive(Debug, PartialEq, FromRepr, Copy, Clone)]
#[repr(u8)]
pub enum Instruction{
    NOP         = 0x0,
    ACONST_NULL = 0x1,
    ICONST0 = 0x3,
    ICONST1 = 0x4,
    ICONST2 = 0x5,
    ICONST3 = 0x6,
    ICONST4 = 0x7,
    ICONST5 = 0x8,
    LCONST0 = 0x9,
    LCONST1 = 0xa,
    FCONST0 = 0xb,
    FCONST1 = 0xc,
    FCONST2 = 0xd,

    ARETURN = 0xb0,
    IRETURN = 0xac,
    LRETURN = 0xad,
    FRETURN = 0xae,
    DRETURN = 0xaf,
    RETURN  = 0xb1,

    BIPUSH(u8)  = 0x10,
    SIPUSH(u16) = 0x11,
    LDC(u8)     = 0x12,
    LDCW(u16)   = 0x13,
    LDC2W(u16)  = 0x14,

    ILOAD(u8)  = 0x15,
    ILOAD0 = 0x1a,
    ILOAD1 = 0x1b,
    ILOAD2 = 0x1c,
    ILOAD3 = 0x1d,

    LLOAD0 = 0x1e,
    LLOAD1 = 0x1f,
    LLOAD2 = 0x20,
    LLOAD3 = 0x21,

    FLOAD0 = 0x22,
    FLOAD1 = 0x23,
    FLOAD2 = 0x24,
    FLOAD3 = 0x25,

    DLOAD0 = 0x26,
    DLOAD1 = 0x27,
    DLOAD2 = 0x28,
    DLOAD3 = 0x29,

    ALOAD(u8)  = 0x19,
    ALOAD0     = 0x2a,
    ALOAD1     = 0x2b,
    ALOAD2     = 0x2c,
    ALOAD3     = 0x2d,
    IALOAD     = 0x2e,
    AALOAD     = 0x32,

    ISTORE(u8) = 0x36,
    ISTORE0    = 0x3b,
    ISTORE1    = 0x3c,
    ISTORE2    = 0x3d,
    ISTORE3    = 0x3e,

    LSTORE0    = 0x3f,
    LSTORE1    = 0x40,
    LSTORE2    = 0x41,
    LSTORE3    = 0x42,

    ASTORE(u8) = 0x3a,
    ASTORE0    = 0x4b,
    ASTORE1    = 0x4c,
    ASTORE2    = 0x4d,
    ASTORE3    = 0x4e,
    IASTORE    = 0x4f,
    AASTORE    = 0x53,
    CASTORE    = 0x55,

    IADD = 0x60,
    LADD = 0x61,
    ISUB = 0x64,
    IMUL = 0x68,
    FMUL = 0x6a,

    ISHL         = 0x78,
    ISHR         = 0x7a,
    IUSHR        = 0x7c,
    LUSHR        = 0x7d,
    IAND         = 0x7e,
    LAND         = 0x7f,
    IOR          = 0x80,
    IXOR         = 0x82,
    IINC(u8, i8) = 0x84,

    GETSTATIC(u16) = 0xb2,
    PUTSTATIC(u16) = 0xb3,
    GETFIELD(u16)  = 0xb4,
    PUTFIELD(u16)  = 0xb5,

    INVOKEVIRTUAL(u16) = 0xb6,
    INVOKESPECIAL(u16) = 0xb7,
    INVOKESTATIC(u16)  = 0xb8,
    INVOKEINTERFACE(u16, u8, u8) = 0xb9,

    ARRAYLENGTH    = 0xbe,

    IF_ICMPNE(u16) = 0xa0,
    IF_ICMPLT(u16) = 0xa1,
    IF_ICMPGE(u16) = 0xa2,
    IF_ICMPLE(u16) = 0xa4,

    IF_ACMPEQ(u16) = 0xa5,
    IF_ACMPNE(u16) = 0xa6,
    IFNULL(u16)    = 0xc6,
    IFNONNULL(u16) = 0xc7,
    IFEQ(u16) = 0x99,
    IFNE(u16) = 0x9a,
    IFLT(u16) = 0x9b,
    IFGE(u16) = 0x9c,
    IFGT(u16) = 0x9d,
    IFLE(u16) = 0x9e,
    LCMP = 0x94,
    FCMPL = 0x95,
    FCMPG = 0x96,
    GOTO(u16) = 0xa7,

    NEW(u16)       = 0xbb,
    NEWARRAY(u8)   = 0xbc,
    ANEWARRAY(u16) = 0xbd,
    POP            = 0x57,
    DUP            = 0x59,

    I2L = 0x85,
    I2F = 0x86,
    L2I = 0x88,
    F2I = 0x8b,
    D2I = 0x8e,

    ATHROW = 0xbf,

    CHECKCAST(u16)= 0xc0,
    MONITORENTER  = 0xc2,
    MONITOREXIT   = 0xc3,

}