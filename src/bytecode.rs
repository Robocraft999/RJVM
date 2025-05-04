use strum_macros::FromRepr;
use crate::bytecode::Instruction::*;
use crate::error::ClassParseError;
use crate::vm::VmError;

pub fn printable_instructions(code_bytes: &Vec<u8>) -> Vec<Instruction>{
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

fn parse_i4(code_bytes: &Vec<u8>, pc: &mut usize) -> Result<i32, VmError>{
    let res = i32::from_be_bytes([parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?]);
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
            TABLESWITCH(_, _, _, _) => {
                let instruction_pc = pc - 1;
                //let padding = pc % 4;
                //ti: 4 -> pc = 5 -> padding = 1 -> dbi = 5+1=6  X
                //ti: 5 -> pc = 6 -> padding = 2 -> dbi = 6+2=8  J
                //ti: 6 -> pc = 7 -> padding = 3 -> dbi = 7+3=10 X
                //ti: 7 -> pc = 8 -> padding = 0 -> dbi = 8+0=8  J

                let padding = (4 - (pc % 4)) % 4;
                //ti: 4 -> pc = 5 -> padding = 3 -> dbi = 5+3=8  J
                //ti: 5 -> pc = 6 -> padding = 2 -> dbi = 6+2=8  J
                //ti: 6 -> pc = 7 -> padding = 1 -> dbi = 7+1=8  J
                //ti: 7 -> pc = 8 -> padding = 0 -> dbi = 8+0=8  J
                for _ in 0..padding{
                    parse_u1(code_bytes, &mut pc)?;
                }
                let default = parse_i4(code_bytes, &mut pc)?;
                let low     = parse_i4(code_bytes, &mut pc)?;
                let high    = parse_i4(code_bytes, &mut pc)?;
                let mut offsets = Vec::new();
                for _ in 0..(high-low +1){
                    offsets.push(parse_i4(code_bytes, &mut pc)?);
                }

                TABLESWITCH(default, low, high, offsets)
            }
            LOOKUPSWITCH(_, _) => {
                let padding = (4 - (pc % 4)) % 4;
                let instruction_pc = pc - 1;
                for _ in 0..padding{
                    parse_u1(code_bytes, &mut pc)?;
                }

                let default = (instruction_pc as i32 + parse_i4(code_bytes, &mut pc)?);
                let npairs = parse_i4(code_bytes, &mut pc)?;

                let mut offsets = Vec::new();
                for _ in 0..npairs{
                    offsets.push(parse_i4(code_bytes, &mut pc)?);
                    //TODO check if this could overflow
                    offsets.push(instruction_pc as i32 + parse_i4(code_bytes, &mut pc)?) ;
                }
                LOOKUPSWITCH(default, offsets)
            }
            INVOKEVIRTUAL(_) => INVOKEVIRTUAL(parse_u2(code_bytes, &mut pc)?),
            INVOKESPECIAL(_) => INVOKESPECIAL(parse_u2(code_bytes, &mut pc)?),
            INVOKESTATIC(_) => INVOKESTATIC(parse_u2(code_bytes, &mut pc)?),
            INVOKEINTERFACE(_, _, _) => INVOKEINTERFACE(parse_u2(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?),
            INVOKEDYNAMIC(_, _, _) => INVOKEDYNAMIC(parse_u2(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?),
            GETSTATIC(_) => GETSTATIC(parse_u2(code_bytes, &mut pc)?),
            PUTSTATIC(_) => PUTSTATIC(parse_u2(code_bytes, &mut pc)?),
            IF_ACMPNE(_) => IF_ACMPNE(parse_offset(code_bytes, &mut pc)?),
            IF_ACMPEQ(_) => IF_ACMPEQ(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPLE(_) => IF_ICMPLE(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPGE(_) => IF_ICMPGE(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPGT(_) => IF_ICMPGT(parse_offset(code_bytes, &mut pc)?),
            IF_ICMPEQ(_) => IF_ICMPEQ(parse_offset(code_bytes, &mut pc)?),
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
            LSTORE(_) => LSTORE(parse_u1(code_bytes, &mut pc)?),
            LLOAD(_) => LLOAD(parse_u1(code_bytes, &mut pc)?),
            FSTORE(_) => FSTORE(parse_u1(code_bytes, &mut pc)?),
            FLOAD(_) => FLOAD(parse_u1(code_bytes, &mut pc)?),
            DSTORE(_) => DSTORE(parse_u1(code_bytes, &mut pc)?),
            DLOAD(_) => DLOAD(parse_u1(code_bytes, &mut pc)?),
            ASTORE(_) => ASTORE(parse_u1(code_bytes, &mut pc)?),
            ALOAD(_) => ALOAD(parse_u1(code_bytes, &mut pc)?),
            NEW(_) => NEW(parse_u2(code_bytes, &mut pc)?),
            ANEWARRAY(_) => ANEWARRAY(parse_u2(code_bytes, &mut pc)?),
            NEWARRAY(_) => NEWARRAY(parse_u1(code_bytes, &mut pc)?),
            IINC(_, _) => IINC(parse_u1(code_bytes, &mut pc)?, parse_i1(code_bytes, &mut pc)?),
            CHECKCAST(_) => CHECKCAST(parse_u2(code_bytes, &mut pc)?),
            INSTANCEOF(_) => INSTANCEOF(parse_u2(code_bytes, &mut pc)?),
            RETURN | IRETURN | ARETURN | DRETURN | LRETURN | FRETURN |
            ALOAD0 | ALOAD1 | ALOAD2 | ALOAD3 | IALOAD | BALOAD | CALOAD | SALOAD | LALOAD | FALOAD | DALOAD | AALOAD |
            LLOAD0 | LLOAD1 | LLOAD2 | LLOAD3 |
            ILOAD0 | ILOAD1 | ILOAD2 | ILOAD3 |
            FLOAD0 | FLOAD1 | FLOAD2 | FLOAD3 |
            DLOAD0 | DLOAD1 | DLOAD2 | DLOAD3 |
            ACONST_NULL |
            ICONST0 | ICONST1 | ICONST2 | ICONST3 | ICONST4 | ICONST5 | ICONSTM1 |
            LCONST0 | LCONST1 |
            FCONST0 | FCONST1 |
            DCONST0 | DCONST1 |
            ISTORE0 | ISTORE1 | ISTORE2 | ISTORE3 |
            ASTORE0 | ASTORE1 | ASTORE2 | ASTORE3 | IASTORE | BASTORE | CASTORE | SASTORE | LASTORE | FASTORE | DASTORE | AASTORE |
            LSTORE0 | LSTORE1 | LSTORE2 | LSTORE3 |
            DSTORE0 | DSTORE1 | DSTORE2 | DSTORE3 |
            DUP | DUPX1 | DUP2 | DUP2X1 | LCMP | ATHROW |
            IADD | LADD | DADD | FADD | ISUB | LSUB | FSUB | DSUB | IMUL | LMUL | FMUL | DMUL | IDIV | LDIV | FDIV | DDIV |
            ARRAYLENGTH | POP | NOP | POP2 |
            LUSHR | LSHR | LSHL | ISHL | ISHR | IUSHR | IOR | LOR | IXOR | LXOR | IAND | LAND | INEG | LNEG | FNEG | IREM |
            MONITORENTER | MONITOREXIT |
            I2L | I2F | I2D | I2B | I2C | I2S | L2I | L2F | L2D | F2I | F2L | F2D | D2I | D2L | D2F |
            FCMPG | FCMPL | DCMPL | DCMPG => instruction,
            _ => unreachable!("Instruction {:?} not initializable", instruction)
        }
    } else {
        unimplemented!("Instruction '0x{:x}' not supported yet", opcode);
    };
    Ok((result, pc))
}

#[derive(Debug, PartialEq, FromRepr, Clone)]
#[repr(u8)]
pub enum Instruction{
    NOP         = 0x0,
    ACONST_NULL = 0x1,
    ICONSTM1 = 0x2,
    ICONST0  = 0x3,
    ICONST1  = 0x4,
    ICONST2  = 0x5,
    ICONST3  = 0x6,
    ICONST4  = 0x7,
    ICONST5  = 0x8,
    LCONST0  = 0x9,
    LCONST1  = 0xa,
    FCONST0  = 0xb,
    FCONST1  = 0xc,
    FCONST2  = 0xd,
    DCONST0  = 0xe,
    DCONST1  = 0xf,

    BIPUSH(u8)  = 0x10,
    SIPUSH(u16) = 0x11,
    LDC(u8)     = 0x12,
    LDCW(u16)   = 0x13,
    LDC2W(u16)  = 0x14,

    ILOAD(u8) = 0x15,
    LLOAD(u8) = 0x16,
    FLOAD(u8) = 0x17,
    DLOAD(u8) = 0x18,
    ALOAD(u8)  = 0x19,

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

    ALOAD0     = 0x2a,
    ALOAD1     = 0x2b,
    ALOAD2     = 0x2c,
    ALOAD3     = 0x2d,
    IALOAD     = 0x2e,
    LALOAD     = 0x2f,
    FALOAD     = 0x30,
    DALOAD     = 0x31,
    AALOAD     = 0x32,
    BALOAD     = 0x33,
    CALOAD     = 0x34,
    SALOAD     = 0x35,

    ISTORE(u8) = 0x36,
    LSTORE(u8) = 0x37,
    FSTORE(u8) = 0x38,
    DSTORE(u8) = 0x39,
    ASTORE(u8) = 0x3a,

    ISTORE0    = 0x3b,
    ISTORE1    = 0x3c,
    ISTORE2    = 0x3d,
    ISTORE3    = 0x3e,

    LSTORE0    = 0x3f,
    LSTORE1    = 0x40,
    LSTORE2    = 0x41,
    LSTORE3    = 0x42,

    FSTORE0    = 0x43,
    FSTORE1    = 0x44,
    FSTORE2    = 0x45,
    FSTORE3    = 0x46,

    DSTORE0    = 0x47,
    DSTORE1    = 0x48,
    DSTORE2    = 0x49,
    DSTORE3    = 0x4a,

    ASTORE0    = 0x4b,
    ASTORE1    = 0x4c,
    ASTORE2    = 0x4d,
    ASTORE3    = 0x4e,

    IASTORE    = 0x4f,
    LASTORE    = 0x50,
    FASTORE    = 0x51,
    DASTORE    = 0x52,
    AASTORE    = 0x53,
    BASTORE    = 0x54,
    CASTORE    = 0x55,
    SASTORE    = 0x56,

    POP        = 0x57,
    POP2       = 0x58,
    DUP        = 0x59,
    DUPX1      = 0x5a,
    DUPX2      = 0x5b,
    DUP2       = 0x5c,
    DUP2X1     = 0x5d,
    DUP2X2     = 0x5e,
    SWAP       = 0x5f,

    IADD = 0x60,
    LADD = 0x61,
    FADD = 0x62,
    DADD = 0x63,

    ISUB = 0x64,
    LSUB = 0x65,
    FSUB = 0x66,
    DSUB = 0x67,

    IMUL = 0x68,
    LMUL = 0x69,
    FMUL = 0x6a,
    DMUL = 0x6b,

    IDIV = 0x6c,
    LDIV = 0x6d,
    FDIV = 0x6e,
    DDIV = 0x6f,

    IREM = 0x70,
    LREM = 0x71,
    FREM = 0x72,
    DREM = 0x73,

    INEG = 0x74,
    LNEG = 0x75,
    FNEG = 0x76,
    DNEG = 0x77,

    ISHL         = 0x78,
    LSHL         = 0x79,
    ISHR         = 0x7a,
    LSHR         = 0x7b,
    IUSHR        = 0x7c,
    LUSHR        = 0x7d,

    IAND         = 0x7e,
    LAND         = 0x7f,
    IOR          = 0x80,
    LOR          = 0x81,
    IXOR         = 0x82,
    LXOR         = 0x83,
    IINC(u8, i8) = 0x84,

    I2L = 0x85,
    I2F = 0x86,
    I2D = 0x87,
    L2I = 0x88,
    L2F = 0x89,
    L2D = 0x8a,
    F2I = 0x8b,
    F2L = 0x8c,
    F2D = 0x8d,
    D2I = 0x8e,
    D2L = 0x8f,
    D2F = 0x90,
    I2B = 0x91,
    I2C = 0x92,
    I2S = 0x93,

    LCMP = 0x94,
    FCMPL = 0x95,
    FCMPG = 0x96,
    DCMPL = 0x97,
    DCMPG = 0x98,

    IFEQ(u16) = 0x99,
    IFNE(u16) = 0x9a,
    IFLT(u16) = 0x9b,
    IFGE(u16) = 0x9c,
    IFGT(u16) = 0x9d,
    IFLE(u16) = 0x9e,

    IF_ICMPEQ(u16) = 0x9f,
    IF_ICMPNE(u16) = 0xa0,
    IF_ICMPLT(u16) = 0xa1,
    IF_ICMPGE(u16) = 0xa2,
    IF_ICMPGT(u16) = 0xa3,
    IF_ICMPLE(u16) = 0xa4,

    IF_ACMPEQ(u16) = 0xa5,
    IF_ACMPNE(u16) = 0xa6,

    GOTO(u16)      = 0xa7,
    JSR(u16)       = 0xa8,
    RET(u8)        = 0xa9,

    TABLESWITCH(i32, i32, i32, Vec<i32>) = 0xaa,
    LOOKUPSWITCH(i32, Vec<i32>)          = 0xab,

    IRETURN  = 0xac,
    LRETURN  = 0xad,
    FRETURN  = 0xae,
    DRETURN  = 0xaf,
    ARETURN  = 0xb0,
    RETURN   = 0xb1,

    GETSTATIC(u16) = 0xb2,
    PUTSTATIC(u16) = 0xb3,
    GETFIELD(u16)  = 0xb4,
    PUTFIELD(u16)  = 0xb5,

    INVOKEVIRTUAL(u16) = 0xb6,
    INVOKESPECIAL(u16) = 0xb7,
    INVOKESTATIC(u16)  = 0xb8,
    INVOKEINTERFACE(u16, u8, u8) = 0xb9,
    INVOKEDYNAMIC(u16, u8, u8) = 0xba,

    NEW(u16)       = 0xbb,
    NEWARRAY(u8)   = 0xbc,
    ANEWARRAY(u16) = 0xbd,

    ARRAYLENGTH    = 0xbe,

    ATHROW         = 0xbf,

    CHECKCAST(u16) = 0xc0,
    INSTANCEOF(u16)= 0xc1,
    MONITORENTER   = 0xc2,
    MONITOREXIT    = 0xc3,

    WIDE(u8, u16, Option<u16>) = 0xc4,
    MULTIANEWARRAY(u16, u8) = 0xc5,

    IFNULL(u16)    = 0xc6,
    IFNONNULL(u16) = 0xc7,

    GOTO_W(u32)    = 0xc8,
    JSR_w(u32)     = 0xc9,

    // 0xca = mnemonic breakpoint
    // 0xfd and 0xff = mnemonics impdep1 and impdep2

}