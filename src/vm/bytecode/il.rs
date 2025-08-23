use std::collections::HashMap;

use crate::{bytecode::{parse_instruction, Instruction}, vm::{bytecode::InstructionBlock, value::Value}};

macro_rules! const_ret {
    ($ret_type:pat, $val:expr, $next:expr, $default:expr) => {
        if let Ok(($ret_type, new_new_pc)) = $next{
            (new_new_pc, InstructionBlock::ConstReturn($val))
        } else {
            $default
        }
    };
}

macro_rules! store_without_pop {
    ($ret_type:pat, $val:expr, $next:expr, $default:expr) => {
        if let Ok(($ret_type, new_new_pc)) = $next{
            (new_new_pc, $val)
        } else {
            $default
        }
    };
}

pub fn get_blocks(bytes: &Vec<u8>) -> Vec<InstructionBlock>{
    let mut blocks = Vec::new();
    let mut intermediate_blocks: HashMap<u16, usize> = HashMap::new();
    let mut jmps: Vec<usize> = Vec::new();
    let mut pc = 0;

    while pc < bytes.len(){
        if let Ok((instruction, new_pc)) = parse_instruction(bytes, pc){
            let current_block_index = blocks.len();
            let next = parse_instruction(bytes, new_pc);
            let (next_pc, block) = match instruction{
                Instruction::GOTO(_) | Instruction::IF_ACMPEQ(_) | Instruction::IF_ACMPNE(_) | 
                Instruction::IF_ICMPEQ(_) | Instruction::IF_ICMPGE(_) | Instruction::IF_ICMPGT(_) |
                Instruction::IF_ICMPLE(_) | Instruction::IF_ICMPLT(_) | Instruction::IF_ICMPNE(_) |
                Instruction::IFEQ(_) | Instruction::IFNE(_) | Instruction::IFGT(_) |
                Instruction::IFLT(_) | Instruction::IFGE(_) | Instruction::IFLE(_) |
                Instruction::IFNULL(_) | Instruction::IFNONNULL(_)
                => {
                    jmps.push(current_block_index);
                    (new_pc, InstructionBlock::Single(instruction))
                },
                //AstoreWithoutPop
                Instruction::ASTORE(idx) => {
                    if let Ok((Instruction::ALOAD(idx2), new_new_pc)) = next{
                        if idx == idx2
                        {(new_new_pc, InstructionBlock::AStoreWithoutPop(idx as usize))} else 
                        {(new_pc, InstructionBlock::Single(instruction))}
                    } else {(new_pc, InstructionBlock::Single(instruction))}
                }
                Instruction::ASTORE0 => store_without_pop!(Instruction::ALOAD0, InstructionBlock::AStoreWithoutPop(0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ASTORE1 => store_without_pop!(Instruction::ALOAD1, InstructionBlock::AStoreWithoutPop(1), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ASTORE2 => store_without_pop!(Instruction::ALOAD2, InstructionBlock::AStoreWithoutPop(2), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ASTORE3 => store_without_pop!(Instruction::ALOAD3, InstructionBlock::AStoreWithoutPop(3), next, (new_pc, InstructionBlock::Single(instruction))),
                //Const Return
                Instruction::ICONST0 => const_ret!(Instruction::IRETURN, Value::Integer(0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONST1 => const_ret!(Instruction::IRETURN, Value::Integer(1), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONST2 => const_ret!(Instruction::IRETURN, Value::Integer(2), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONST3 => const_ret!(Instruction::IRETURN, Value::Integer(3), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONST4 => const_ret!(Instruction::IRETURN, Value::Integer(4), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONST5 => const_ret!(Instruction::IRETURN, Value::Integer(5), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ICONSTM1 => const_ret!(Instruction::IRETURN, Value::Integer(-1), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::LCONST0 => const_ret!(Instruction::LRETURN, Value::Long(0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::LCONST1 => const_ret!(Instruction::LRETURN, Value::Long(1), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::FCONST0 => const_ret!(Instruction::FRETURN, Value::Float(0.0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::FCONST1 => const_ret!(Instruction::FRETURN, Value::Float(1.0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::FCONST2 => const_ret!(Instruction::FRETURN, Value::Float(2.0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::DCONST0 => const_ret!(Instruction::DRETURN, Value::Double(0.0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::DCONST1 => const_ret!(Instruction::DRETURN, Value::Double(1.0), next, (new_pc, InstructionBlock::Single(instruction))),
                Instruction::ACONST_NULL => const_ret!(Instruction::ARETURN, Value::Null, next, (new_pc, InstructionBlock::Single(instruction))),
                _ => {(new_pc, InstructionBlock::Single(instruction))}
            };
            intermediate_blocks.insert(pc as u16, current_block_index);
            blocks.push(block);
            pc = next_pc;
        }
    }
    for index in jmps{
        let block: &InstructionBlock = &blocks[index];
        let corrected_instruction = match block{
            InstructionBlock::Single(Instruction::GOTO(target)) => Instruction::GOTO(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ACMPEQ(target)) => Instruction::IF_ACMPEQ(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ACMPNE(target)) => Instruction::IF_ACMPNE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPEQ(target)) => Instruction::IF_ICMPEQ(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPGE(target)) => Instruction::IF_ICMPGE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPGT(target)) => Instruction::IF_ICMPGT(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPLE(target)) => Instruction::IF_ICMPLE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPLT(target)) => Instruction::IF_ICMPLT(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IF_ICMPNE(target)) => Instruction::IF_ICMPNE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFEQ(target)) => Instruction::IFEQ(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFNE(target)) => Instruction::IFNE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFGT(target)) => Instruction::IFGT(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFLT(target)) => Instruction::IFLT(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFGE(target)) => Instruction::IFGE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFLE(target)) => Instruction::IFLE(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFNULL(target)) => Instruction::IFNULL(intermediate_blocks[target] as u16),
            InstructionBlock::Single(Instruction::IFNONNULL(target)) => Instruction::IFNONNULL(intermediate_blocks[target] as u16),
            _ => unreachable!()
        };
        blocks[index] = InstructionBlock::Single(corrected_instruction)
    }

    blocks
}