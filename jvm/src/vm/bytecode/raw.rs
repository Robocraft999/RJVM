use std::collections::BTreeMap;

use crate::{bytecode::parse_instruction, vm::bytecode::InstructionBlock};

pub fn get_blocks(bytes: &Vec<u8>) -> BTreeMap<u16, InstructionBlock>{
    let mut blocks = BTreeMap::new();
    let mut parse_pc = 0;

    while parse_pc < bytes.len() {
        if let Ok((instruction, new_parse_pc)) = parse_instruction(bytes, parse_pc) {
            blocks.insert(parse_pc as u16, InstructionBlock::Single(instruction));
            parse_pc = new_parse_pc;
        }
    }
    blocks
}