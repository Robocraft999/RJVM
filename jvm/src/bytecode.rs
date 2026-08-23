#![allow(non_camel_case_types)]
use crate::bytecode::Instruction::*;
use crate::error::ClassParseError;
use crate::vm::VmError;
use strum_macros::FromRepr;
use crate::class_file::methods::code::PC;
use crate::vm::result::VMResult;

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

fn parse_u1(code_bytes: &[u8], pc: &mut usize) -> VMResult<u8>{
    let res = *code_bytes.get(*pc).ok_or(VmError::ParseError(ClassParseError::ReadError))?;
    *pc += 1;
    Ok(res)
}

fn parse_i1(code_bytes: &[u8], pc: &mut usize) -> VMResult<i8>{
    Ok(parse_u1(code_bytes, pc)? as i8)
}

fn parse_u2(code_bytes: &[u8], pc: &mut usize) -> VMResult<u16>{
    let res = u16::from_be_bytes([parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?]);
    Ok(res)
}

fn parse_i2(code_bytes: &[u8], pc: &mut usize) -> VMResult<i16>{
    let res = i16::from_be_bytes([parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?]);
    Ok(res)
}

fn parse_i4(code_bytes: &[u8], pc: &mut usize) -> VMResult<i32>{
    let res = i32::from_be_bytes([parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?, parse_u1(code_bytes, pc)?]);
    Ok(res)
}

fn parse_offset(code_bytes: &[u8], pc: &mut usize) -> VMResult<u16>{
    let instruction_pc = *pc - 1;
    let offset = parse_i2(code_bytes, pc)?;
    Ok(((instruction_pc as i16) + offset) as u16)
}

pub fn parse_instruction(code_bytes: &[u8], mut pc: usize) -> Result<(Instruction, usize), VmError> {
    let instruction_pc = pc;
    let opcode = parse_u1(code_bytes, &mut pc)?;
    let result = if let Some(instruction) = BytecodeInstruction::from_repr(opcode){
        match instruction{
            BytecodeInstruction::TABLESWITCH => {
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
                let mut targets = Vec::new();
                for _ in 0..(high-low +1){
                    let offset = parse_i4(code_bytes, &mut pc)?;
                    let target_pc = (instruction_pc as i32 + offset) as PC;
                    targets.push(target_pc);
                }

                let default_pc = (instruction_pc as i32 + default) as PC;
                TABLESWITCH(low, high, default_pc, targets)
            }
            BytecodeInstruction::LOOKUPSWITCH => {
                let padding = (4 - (pc % 4)) % 4;
                for _ in 0..padding{
                    parse_u1(code_bytes, &mut pc)?;
                }

                let default = parse_i4(code_bytes, &mut pc)?;
                let npairs = parse_i4(code_bytes, &mut pc)?;

                let mut targets = Vec::new();
                for _ in 0..npairs{
                    let value = parse_i4(code_bytes, &mut pc)?;
                    let target_pc = (instruction_pc as i32 + parse_i4(code_bytes, &mut pc)?) as PC;
                    targets.push((value, target_pc));
                }
                let default_pc = (instruction_pc as i32 + default) as PC;
                LOOKUPSWITCH(default_pc, targets)
            }
            BytecodeInstruction::WIDE => {
                let ins = parse_u1(code_bytes, &mut pc)?;
                let index = parse_u2(code_bytes, &mut pc)?;
                //IINC
                let value = if ins == 0x84{
                    Some(parse_u2(code_bytes, &mut pc)?)
                } else {
                    None
                };
                WIDE(ins, index, value)
            }
            BytecodeInstruction::INVOKEVIRTUAL => INVOKEVIRTUAL(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::INVOKESPECIAL => INVOKESPECIAL(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::INVOKESTATIC => INVOKESTATIC(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::INVOKEINTERFACE => {
                let index = parse_u2(code_bytes, &mut pc)?;
                let arg_count = parse_u1(code_bytes, &mut pc)?;
                let _ = parse_u1(code_bytes, &mut pc)?;
                INVOKEINTERFACE(index, arg_count)
            }
            BytecodeInstruction::INVOKEDYNAMIC => {
                let index = parse_u2(code_bytes, &mut pc)?;
                let _ = parse_u1(code_bytes, &mut pc)?;
                let _ = parse_u1(code_bytes, &mut pc)?;
                INVOKEDYNAMIC(index)
            }
            BytecodeInstruction::GETSTATIC => GETSTATIC(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::PUTSTATIC => PUTSTATIC(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ACMPNE => IF_ACMPNE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ACMPEQ => IF_ACMPEQ(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPLE => IF_ICMPLE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPGE => IF_ICMPGE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPGT => IF_ICMPGT(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPEQ => IF_ICMPEQ(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPLT => IF_ICMPLT(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IF_ICMPNE => IF_ICMPNE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFNONNULL => IFNONNULL(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFNULL => IFNULL(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFEQ => IFEQ(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFNE => IFNE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFGE => IFGE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFGT => IFGT(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFLT => IFLT(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::IFLE => IFLE(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::GOTO => GOTO(parse_offset(code_bytes, &mut pc)?),
            BytecodeInstruction::LDC => LDC(parse_u1(code_bytes, &mut pc)? as u16),
            BytecodeInstruction::LDCW => LDC(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::LDC2W => LDC2(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::BIPUSH => ICONST(parse_i1(code_bytes, &mut pc)? as i32),
            BytecodeInstruction::SIPUSH => ICONST(parse_i2(code_bytes, &mut pc)? as i32),
            BytecodeInstruction::GETFIELD => GETFIELD(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::PUTFIELD => PUTFIELD(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::NEW => NEW(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::ANEWARRAY => ANEWARRAY(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::MULTIANEWARRAY => MULTIANEWARRAY(parse_u2(code_bytes, &mut pc)?, parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::NEWARRAY => NEWARRAY(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::IINC => IINC(parse_u1(code_bytes, &mut pc)?, parse_i1(code_bytes, &mut pc)?),
            BytecodeInstruction::CHECKCAST => CHECKCAST(parse_u2(code_bytes, &mut pc)?),
            BytecodeInstruction::INSTANCEOF => INSTANCEOF(parse_u2(code_bytes, &mut pc)?),

            BytecodeInstruction::IRETURN => IRETURN,
            BytecodeInstruction::LRETURN => LRETURN,
            BytecodeInstruction::FRETURN => FRETURN,
            BytecodeInstruction::DRETURN => DRETURN,
            BytecodeInstruction::ARETURN => ARETURN,
            BytecodeInstruction::RETURN => RETURN,

            BytecodeInstruction::ALOAD =>  ALOAD(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::ALOAD0 => ALOAD(0),
            BytecodeInstruction::ALOAD1 => ALOAD(1),
            BytecodeInstruction::ALOAD2 => ALOAD(2),
            BytecodeInstruction::ALOAD3 => ALOAD(3),
            BytecodeInstruction::IALOAD => IALOAD,
            BytecodeInstruction::LALOAD => LALOAD,
            BytecodeInstruction::FALOAD => FALOAD,
            BytecodeInstruction::DALOAD => DALOAD,
            BytecodeInstruction::AALOAD => AALOAD,
            BytecodeInstruction::BALOAD => BALOAD,
            BytecodeInstruction::CALOAD => CALOAD,
            BytecodeInstruction::SALOAD => SALOAD,

            BytecodeInstruction::ILOAD => ILOAD(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::ILOAD0 => ILOAD(0),
            BytecodeInstruction::ILOAD1 => ILOAD(1),
            BytecodeInstruction::ILOAD2 => ILOAD(2),
            BytecodeInstruction::ILOAD3 => ILOAD(3),

            BytecodeInstruction::LLOAD =>  LLOAD(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::LLOAD0 => LLOAD(0),
            BytecodeInstruction::LLOAD1 => LLOAD(1),
            BytecodeInstruction::LLOAD2 => LLOAD(2),
            BytecodeInstruction::LLOAD3 => LLOAD(3),

            BytecodeInstruction::FLOAD =>  FLOAD(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::FLOAD0 => FLOAD(0),
            BytecodeInstruction::FLOAD1 => FLOAD(1),
            BytecodeInstruction::FLOAD2 => FLOAD(2),
            BytecodeInstruction::FLOAD3 => FLOAD(3),

            BytecodeInstruction::DLOAD =>  DLOAD(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::DLOAD0 => DLOAD(0),
            BytecodeInstruction::DLOAD1 => DLOAD(1),
            BytecodeInstruction::DLOAD2 => DLOAD(2),
            BytecodeInstruction::DLOAD3 => DLOAD(3),

            BytecodeInstruction::ACONST_NULL => ACONST_NULL,
            BytecodeInstruction::ICONSTM1 => ICONST(-1),
            BytecodeInstruction::ICONST0 => ICONST(0),
            BytecodeInstruction::ICONST1 => ICONST(1),
            BytecodeInstruction::ICONST2 => ICONST(2),
            BytecodeInstruction::ICONST3 => ICONST(3),
            BytecodeInstruction::ICONST4 => ICONST(4),
            BytecodeInstruction::ICONST5 => ICONST(5),
            BytecodeInstruction::LCONST0 => LCONST(0),
            BytecodeInstruction::LCONST1 => LCONST(1),
            BytecodeInstruction::FCONST0 => FCONST(0.0),
            BytecodeInstruction::FCONST1 => FCONST(1.0),
            BytecodeInstruction::FCONST2 => FCONST(2.0),
            BytecodeInstruction::DCONST0 => DCONST(0.0),
            BytecodeInstruction::DCONST1 => DCONST(1.0),

            BytecodeInstruction::ASTORE =>  ASTORE(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::ASTORE0 => ASTORE(0),
            BytecodeInstruction::ASTORE1 => ASTORE(1),
            BytecodeInstruction::ASTORE2 => ASTORE(2),
            BytecodeInstruction::ASTORE3 => ASTORE(3),
            BytecodeInstruction::IASTORE => IASTORE,
            BytecodeInstruction::LASTORE => LASTORE,
            BytecodeInstruction::FASTORE => FASTORE,
            BytecodeInstruction::DASTORE => DASTORE,
            BytecodeInstruction::AASTORE => AASTORE,
            BytecodeInstruction::BASTORE => BASTORE,
            BytecodeInstruction::CASTORE => CASTORE,
            BytecodeInstruction::SASTORE => SASTORE,

            BytecodeInstruction::ISTORE =>  ISTORE(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::ISTORE0 => ISTORE(0),
            BytecodeInstruction::ISTORE1 => ISTORE(1),
            BytecodeInstruction::ISTORE2 => ISTORE(2),
            BytecodeInstruction::ISTORE3 => ISTORE(3),

            BytecodeInstruction::LSTORE =>  LSTORE(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::LSTORE0 => LSTORE(0),
            BytecodeInstruction::LSTORE1 => LSTORE(1),
            BytecodeInstruction::LSTORE2 => LSTORE(2),
            BytecodeInstruction::LSTORE3 => LSTORE(3),

            BytecodeInstruction::FSTORE =>  FSTORE(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::FSTORE0 => FSTORE(0),
            BytecodeInstruction::FSTORE1 => FSTORE(1),
            BytecodeInstruction::FSTORE2 => FSTORE(2),
            BytecodeInstruction::FSTORE3 => FSTORE(3),

            BytecodeInstruction::DSTORE =>  DSTORE(parse_u1(code_bytes, &mut pc)?),
            BytecodeInstruction::DSTORE0 => DSTORE(0),
            BytecodeInstruction::DSTORE1 => DSTORE(1),
            BytecodeInstruction::DSTORE2 => DSTORE(2),
            BytecodeInstruction::DSTORE3 => DSTORE(3),

            BytecodeInstruction::DUP => DUP,
            BytecodeInstruction::DUPX1 => DUPX1,
            BytecodeInstruction::DUPX2 => DUPX2,
            BytecodeInstruction::DUP2 => DUP2,
            BytecodeInstruction::DUP2X1 => DUP2X1,
            BytecodeInstruction::DUP2X2 => DUP2X2,

            BytecodeInstruction::NOP => NOP,
            BytecodeInstruction::POP => POP,
            BytecodeInstruction::POP2 => POP2,
            BytecodeInstruction::SWAP => SWAP,

            BytecodeInstruction::IADD => IADD,
            BytecodeInstruction::LADD => LADD,
            BytecodeInstruction::FADD => FADD,
            BytecodeInstruction::DADD => DADD,

            BytecodeInstruction::ISUB => ISUB,
            BytecodeInstruction::LSUB => LSUB,
            BytecodeInstruction::FSUB => FSUB,
            BytecodeInstruction::DSUB => DSUB,

            BytecodeInstruction::IMUL => IMUL,
            BytecodeInstruction::LMUL => LMUL,
            BytecodeInstruction::FMUL => FMUL,
            BytecodeInstruction::DMUL => DMUL,

            BytecodeInstruction::IDIV => IDIV,
            BytecodeInstruction::LDIV => LDIV,
            BytecodeInstruction::FDIV => FDIV,
            BytecodeInstruction::DDIV => DDIV,

            BytecodeInstruction::IREM => IREM,
            BytecodeInstruction::LREM => LREM,
            BytecodeInstruction::FREM => FREM,
            BytecodeInstruction::DREM => DREM,

            BytecodeInstruction::INEG => INEG,
            BytecodeInstruction::LNEG => LNEG,
            BytecodeInstruction::FNEG => FNEG,
            BytecodeInstruction::DNEG => DNEG,

            BytecodeInstruction::ISHL => ISHL,
            BytecodeInstruction::LSHL => LSHL,
            BytecodeInstruction::ISHR => ISHR,
            BytecodeInstruction::LSHR => LSHR,
            BytecodeInstruction::IUSHR => IUSHR,
            BytecodeInstruction::LUSHR => LUSHR,

            BytecodeInstruction::IAND => IAND,
            BytecodeInstruction::LAND => LAND,
            BytecodeInstruction::IOR => IOR,
            BytecodeInstruction::LOR => LOR,
            BytecodeInstruction::IXOR => IXOR,
            BytecodeInstruction::LXOR => LXOR,

            BytecodeInstruction::I2L => I2L,
            BytecodeInstruction::I2F => I2F,
            BytecodeInstruction::I2D => I2D,
            BytecodeInstruction::L2I => L2I,
            BytecodeInstruction::L2F => L2F,
            BytecodeInstruction::L2D => L2D,
            BytecodeInstruction::F2I => F2I,
            BytecodeInstruction::F2L => F2L,
            BytecodeInstruction::F2D => F2D,
            BytecodeInstruction::D2I => D2I,
            BytecodeInstruction::D2L => D2L,
            BytecodeInstruction::D2F => D2F,
            BytecodeInstruction::I2B => I2B,
            BytecodeInstruction::I2C => I2C,
            BytecodeInstruction::I2S => I2S,

            BytecodeInstruction::LCMP => LCMP,
            BytecodeInstruction::FCMPL => FCMPL,
            BytecodeInstruction::FCMPG => FCMPG,
            BytecodeInstruction::DCMPL => DCMPL,
            BytecodeInstruction::DCMPG => DCMPG,

            BytecodeInstruction::JSR => JSR(parse_i2(code_bytes, &mut pc)?),
            BytecodeInstruction::RET => RET(parse_u1(code_bytes, &mut pc)?),

            BytecodeInstruction::ARRAYLENGTH => ARRAYLENGTH,
            BytecodeInstruction::ATHROW => ATHROW,
            BytecodeInstruction::MONITORENTER => MONITORENTER,
            BytecodeInstruction::MONITOREXIT => MONITOREXIT,
            BytecodeInstruction::GOTO_W => {
                let offset = parse_i4(code_bytes, &mut pc)?;
                GOTO_W((instruction_pc as i32 + offset) as u32)
            }
            BytecodeInstruction::JSR_w => JSR_W(parse_i4(code_bytes, &mut pc)?),
        }
    } else {
        unimplemented!("Instruction '0x{:x}' not supported yet", opcode);
    };
    Ok((result, pc))
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction{
    NOP,
    ACONST_NULL,
    ICONST(i32),
    LCONST(i64),
    FCONST(f32),
    DCONST(f64),

    LDC(u16),
    LDC2(u16),

    ILOAD(u8),
    LLOAD(u8),
    FLOAD(u8),
    DLOAD(u8),
    ALOAD(u8),

    IALOAD,
    LALOAD,
    FALOAD,
    DALOAD,
    AALOAD,
    BALOAD,
    CALOAD,
    SALOAD,

    ISTORE(u8),
    LSTORE(u8),
    FSTORE(u8),
    DSTORE(u8),
    ASTORE(u8),

    IASTORE,
    LASTORE,
    FASTORE,
    DASTORE,
    AASTORE,
    BASTORE,
    CASTORE,
    SASTORE,

    POP,
    POP2,
    DUP,
    DUPX1,
    DUPX2,
    DUP2,
    DUP2X1,
    DUP2X2,
    SWAP,

    IADD,
    LADD,
    FADD,
    DADD,

    ISUB,
    LSUB,
    FSUB,
    DSUB,

    IMUL,
    LMUL,
    FMUL,
    DMUL,

    IDIV,
    LDIV,
    FDIV,
    DDIV,

    IREM,
    LREM,
    FREM,
    DREM,

    INEG,
    LNEG,
    FNEG,
    DNEG,

    ISHL,
    LSHL,
    ISHR,
    LSHR,
    IUSHR,
    LUSHR,

    IAND,
    LAND,
    IOR,
    LOR,
    IXOR,
    LXOR,
    IINC(u8, i8),

    I2L,
    I2F,
    I2D,
    L2I,
    L2F,
    L2D,
    F2I,
    F2L,
    F2D,
    D2I,
    D2L,
    D2F,
    I2B,
    I2C,
    I2S,

    LCMP,
    FCMPL,
    FCMPG,
    DCMPL,
    DCMPG,

    IFEQ(PC),
    IFNE(PC),
    IFLT(PC),
    IFGE(PC),
    IFGT(PC),
    IFLE(PC),

    IF_ICMPEQ(PC),
    IF_ICMPNE(PC),
    IF_ICMPLT(PC),
    IF_ICMPGE(PC),
    IF_ICMPGT(PC),
    IF_ICMPLE(PC),

    IF_ACMPEQ(PC),
    IF_ACMPNE(PC),

    GOTO(PC),
    JSR(i16),
    RET(u8),

    TABLESWITCH(i32, i32, PC, Vec<PC>),
    LOOKUPSWITCH(PC, Vec<(i32, PC)>),

    IRETURN,
    LRETURN,
    FRETURN,
    DRETURN,
    ARETURN,
    RETURN,

    GETSTATIC(u16),
    PUTSTATIC(u16),
    GETFIELD(u16),
    PUTFIELD(u16),

    INVOKEVIRTUAL(u16),
    INVOKESPECIAL(u16),
    INVOKESTATIC(u16),
    INVOKEINTERFACE(u16, u8),
    INVOKEDYNAMIC(u16),

    NEW(u16),
    NEWARRAY(u8),
    ANEWARRAY(u16),

    ARRAYLENGTH,

    ATHROW,

    CHECKCAST(u16),
    INSTANCEOF(u16),
    MONITORENTER,
    MONITOREXIT,

    WIDE(u8, u16, Option<u16>),
    MULTIANEWARRAY(u16, u8),

    IFNULL(PC),
    IFNONNULL(PC),

    GOTO_W(u32),
    JSR_W(i32),
}

#[derive(Debug, PartialEq, FromRepr, Clone)]
#[repr(u8)]
pub enum BytecodeInstruction {
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

    BIPUSH  = 0x10,
    SIPUSH  = 0x11,
    LDC     = 0x12,
    LDCW    = 0x13,
    LDC2W   = 0x14,

    ILOAD = 0x15,
    LLOAD = 0x16,
    FLOAD = 0x17,
    DLOAD = 0x18,
    ALOAD = 0x19,

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

    ALOAD0 = 0x2a,
    ALOAD1 = 0x2b,
    ALOAD2 = 0x2c,
    ALOAD3 = 0x2d,
    IALOAD = 0x2e,
    LALOAD = 0x2f,
    FALOAD = 0x30,
    DALOAD = 0x31,
    AALOAD = 0x32,
    BALOAD = 0x33,
    CALOAD = 0x34,
    SALOAD = 0x35,

    ISTORE = 0x36,
    LSTORE = 0x37,
    FSTORE = 0x38,
    DSTORE = 0x39,
    ASTORE = 0x3a,

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

    ISHL  = 0x78,
    LSHL  = 0x79,
    ISHR  = 0x7a,
    LSHR  = 0x7b,
    IUSHR = 0x7c,
    LUSHR = 0x7d,

    IAND  = 0x7e,
    LAND  = 0x7f,
    IOR   = 0x80,
    LOR   = 0x81,
    IXOR  = 0x82,
    LXOR  = 0x83,
    IINC  = 0x84,

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

    IFEQ = 0x99,
    IFNE = 0x9a,
    IFLT = 0x9b,
    IFGE = 0x9c,
    IFGT = 0x9d,
    IFLE = 0x9e,

    IF_ICMPEQ = 0x9f,
    IF_ICMPNE = 0xa0,
    IF_ICMPLT = 0xa1,
    IF_ICMPGE = 0xa2,
    IF_ICMPGT = 0xa3,
    IF_ICMPLE = 0xa4,

    IF_ACMPEQ = 0xa5,
    IF_ACMPNE = 0xa6,

    GOTO      = 0xa7,
    JSR       = 0xa8,
    RET       = 0xa9,

    TABLESWITCH  = 0xaa,
    LOOKUPSWITCH = 0xab,

    IRETURN  = 0xac,
    LRETURN  = 0xad,
    FRETURN  = 0xae,
    DRETURN  = 0xaf,
    ARETURN  = 0xb0,
    RETURN   = 0xb1,

    GETSTATIC = 0xb2,
    PUTSTATIC = 0xb3,
    GETFIELD  = 0xb4,
    PUTFIELD  = 0xb5,

    INVOKEVIRTUAL = 0xb6,
    INVOKESPECIAL = 0xb7,
    INVOKESTATIC  = 0xb8,
    INVOKEINTERFACE = 0xb9,
    INVOKEDYNAMIC = 0xba,

    NEW       = 0xbb,
    NEWARRAY   = 0xbc,
    ANEWARRAY = 0xbd,

    ARRAYLENGTH    = 0xbe,

    ATHROW         = 0xbf,

    CHECKCAST  = 0xc0,
    INSTANCEOF = 0xc1,
    MONITORENTER   = 0xc2,
    MONITOREXIT    = 0xc3,

    WIDE = 0xc4,
    MULTIANEWARRAY = 0xc5,

    IFNULL    = 0xc6,
    IFNONNULL = 0xc7,

    GOTO_W    = 0xc8,
    JSR_w     = 0xc9,

    // 0xca = mnemonic breakpoint
    // 0xfd and 0xff = mnemonics impdep1 and impdep2
}