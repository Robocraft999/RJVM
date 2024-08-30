use strum_macros::FromRepr;

use crate::bytecode::Instruction::*;
use crate::bytes::{parse_u1, parse_u2};

fn read_u2<I: Iterator<Item=u8>>(bytes: &mut I, bytes_read: &mut u16) -> u16{
    *bytes_read += 2;
    parse_u2(bytes).unwrap()
}

fn read_u1<I: Iterator<Item=u8>>(bytes: &mut I, bytes_read: &mut u16) -> u8{
    *bytes_read += 1;
    parse_u1(bytes).unwrap()
}

pub fn instructions_from_bytes(code_bytes: Vec<u8>) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut byte_iter = code_bytes.into_iter();
    let mut data_bytes_parsed = 0;
    while let Some(byte) = byte_iter.next(){
        if let Some(instruction) = Instruction::from_repr(byte){
            instructions.push(match instruction {
                INVOKEVIRTUAL(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    INVOKEVIRTUAL(index)
                }
                INVOKESPECIAL(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    INVOKESPECIAL(index)
                }
                INVOKESTATIC(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    INVOKESTATIC(index)
                }
                GETSTATIC(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    GETSTATIC(index)
                }
                PUTSTATIC(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    PUTSTATIC(index)
                }
                IF_ACMPNE(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    let v: Vec<u8> = byte_iter.clone().take(offset as usize-2).collect();
                    let new_offset = instructions_from_bytes(v).len() as u16;
                    IF_ACMPNE(new_offset)
                }
                IF_ICMPLE(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IF_ICMPLE(offset)
                }
                IF_ICMPGE(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IF_ICMPGE(offset)
                }
                IFEQ(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IFEQ(offset)
                }
                IFNE(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IFNE(offset)
                }
                IFGE(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IFGE(offset)
                }
                IFLT(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    IFLT(offset)
                }
                GOTO(_) => {
                    let offset = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    let v: Vec<u8> = byte_iter.clone().take(offset as usize-2).collect();
                    let new_offset = instructions_from_bytes(v).len() as u16;
                    GOTO(new_offset)
                }
                LDC(_) => {
                    let index = read_u1(&mut byte_iter, &mut data_bytes_parsed);
                    LDC(index)
                }
                BIPUSH(_) => {
                    let value = read_u1(&mut byte_iter, &mut data_bytes_parsed);
                    BIPUSH(value)
                }
                GETFIELD(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    GETFIELD(index)
                }
                PUTFIELD(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    PUTFIELD(index)
                }
                ISTORE(_) => {
                    let value = read_u1(&mut byte_iter, &mut data_bytes_parsed);
                    ISTORE(value)
                }
                ILOAD(_) => {
                    let value = read_u1(&mut byte_iter, &mut data_bytes_parsed);
                    ILOAD(value)
                }
                LDC2W(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    LDC2W(index)
                }
                NEW(_) => {
                    let index = read_u2(&mut byte_iter, &mut data_bytes_parsed);
                    NEW(index)
                }
                RETURN | IRETURN | ARETURN | DRETURN => instruction,
                ALOAD0 | ALOAD1 | ALOAD2 | LLOAD1 | ILOAD1 | ILOAD2 | ILOAD3 => instruction,
                ICONST0 | ICONST1 | ICONST5 | LCONST0 | LCONST1 => instruction,
                ISTORE0 | ISTORE1 | ISTORE2 | ISTORE3 | ASTORE1 | ASTORE2 | LSTORE1 => instruction,
                DUP | LCMP | ATHROW | LADD | IADD | ISUB | IMUL => instruction,
                D2I => instruction,
                _ => unreachable!("Instruction {:?} not initializable", instruction)
            });
        } else {
            unimplemented!("Instruction '{:x}' not supported yet", byte);
        }
    }

    instructions
}

#[derive(Debug, PartialEq, FromRepr, Copy, Clone)]
#[repr(u8)]
pub enum Instruction{
    ICONST0 = 0x3,
    ICONST1 = 0x4,
    ICONST5 = 0x8,
    LCONST0 = 0x9,
    LCONST1 = 0xa,

    ARETURN = 0xb0,
    DRETURN = 0xaf,
    IRETURN = 0xac,
    RETURN = 0xb1,

    BIPUSH(u8) = 0x10,
    LDC(u8) = 0x12,
    LDC2W(u16) = 0x14,
    ILOAD(u8) = 0x15,
    ILOAD0 = 0x1a,
    ILOAD1 = 0x1b,
    ILOAD2 = 0x1c,
    ILOAD3 = 0x1d,
    LLOAD1 = 0x1f,
    ALOAD0 = 0x2a,
    ALOAD1 = 0x2b,
    ALOAD2 = 0x2c,
    ALOAD3 = 0x2d,

    ISTORE(u8) = 0x36,
    ISTORE0    = 0x3b,
    ISTORE1    = 0x3c,
    ISTORE2    = 0x3d,
    ISTORE3    = 0x3e,
    LSTORE1    = 0x40,
    ASTORE0    = 0x4b,
    ASTORE1    = 0x4c,
    ASTORE2    = 0x4d,
    ASTORE3    = 0x4e,

    IADD = 0x60,
    LADD = 0x61,
    ISUB = 0x64,
    IMUL = 0x68,

    GETSTATIC(u16) = 0xb2,
    PUTSTATIC(u16) = 0xb3,
    GETFIELD(u16) = 0xb4,
    PUTFIELD(u16) = 0xb5,

    INVOKEVIRTUAL(u16) = 0xb6,
    INVOKESPECIAL(u16) = 0xb7,
    INVOKESTATIC(u16) = 0xb8,

    IF_ICMPGE(u16) = 0xa2,
    IF_ICMPLE(u16) = 0xa4,
    IF_ACMPNE(u16) = 0xa6,
    IFEQ(u16) = 0x99,
    IFNE(u16) = 0x9a,
    IFLT(u16) = 0x9b,
    IFGE(u16) = 0x9c,
    LCMP = 0x94,
    GOTO(u16) = 0xa7,

    NEW(u16) = 0xbb,
    DUP = 0x59,

    D2I = 0x8e,

    ATHROW = 0xbf,
}