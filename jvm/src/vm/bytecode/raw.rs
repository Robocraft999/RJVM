use std::{collections::HashMap};

use crate::{bytecode::{parse_instruction, Instruction}, vm::bytecode::InstructionBlock};

pub fn get_blocks(bytes: &Vec<u8>) -> Vec<InstructionBlock>{
    let mut instructions = Vec::new();
    let mut intermediate_blocks: HashMap<u16, usize> = HashMap::new();
    let mut jmps: Vec<usize> = Vec::new();
    let mut pc = 0;

    while pc < bytes.len(){
        if let Ok((instruction, new_pc)) = parse_instruction(bytes, pc){
            let current_block_index = instructions.len();
            match instruction{
                Instruction::GOTO(_) | Instruction::IF_ACMPEQ(_) | Instruction::IF_ACMPNE(_) | 
                Instruction::IF_ICMPEQ(_) | Instruction::IF_ICMPGE(_) | Instruction::IF_ICMPGT(_) |
                Instruction::IF_ICMPLE(_) | Instruction::IF_ICMPLT(_) | Instruction::IF_ICMPNE(_) |
                Instruction::IFEQ(_) | Instruction::IFNE(_) | Instruction::IFGT(_) |
                Instruction::IFLT(_) | Instruction::IFGE(_) | Instruction::IFLE(_) |
                Instruction::IFNULL(_) | Instruction::IFNONNULL(_)
                => {jmps.push(current_block_index);},
                _ => {}
            }
            intermediate_blocks.insert(pc as u16, current_block_index);
            instructions.push(instruction);
            pc = new_pc;
        }
    }
    for index in jmps{
        let instruction: &Instruction = &instructions[index];
        let corrected_instruction = match instruction{
            Instruction::GOTO(target) => Instruction::GOTO(intermediate_blocks[target] as u16),
            Instruction::IF_ACMPEQ(target) => Instruction::IF_ACMPEQ(intermediate_blocks[target] as u16),
            Instruction::IF_ACMPNE(target) => Instruction::IF_ACMPNE(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPEQ(target) => Instruction::IF_ICMPEQ(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPGE(target) => Instruction::IF_ICMPGE(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPGT(target) => Instruction::IF_ICMPGT(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPLE(target) => Instruction::IF_ICMPLE(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPLT(target) => Instruction::IF_ICMPLT(intermediate_blocks[target] as u16),
            Instruction::IF_ICMPNE(target) => Instruction::IF_ICMPNE(intermediate_blocks[target] as u16),
            Instruction::IFEQ(target) => Instruction::IFEQ(intermediate_blocks[target] as u16),
            Instruction::IFNE(target) => Instruction::IFNE(intermediate_blocks[target] as u16),
            Instruction::IFGT(target) => Instruction::IFGT(intermediate_blocks[target] as u16),
            Instruction::IFLT(target) => Instruction::IFLT(intermediate_blocks[target] as u16),
            Instruction::IFGE(target) => Instruction::IFGE(intermediate_blocks[target] as u16),
            Instruction::IFLE(target) => Instruction::IFLE(intermediate_blocks[target] as u16),
            Instruction::IFNULL(target) => Instruction::IFNULL(intermediate_blocks[target] as u16),
            Instruction::IFNONNULL(target) => Instruction::IFNONNULL(intermediate_blocks[target] as u16),
            _ => unreachable!()
        };
        instructions[index] = corrected_instruction
    }

    instructions.into_iter().map(InstructionBlock::Single).collect()
}