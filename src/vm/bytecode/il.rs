use std::collections::HashMap;

use crate::{bytecode::{parse_instruction, Instruction}, vm::{bytecode::InstructionBlock, value::Value}};

macro_rules! const_ret {
    ($ret_type:pat, $val:expr, $next:expr, $default:expr) => {
        if let Some($ret_type) = $next{
            (2, InstructionBlock::ConstReturn($val))
        } else {
            (1, $default)
        }
    };
}

macro_rules! store_without_pop {
    ($ret_type:pat, $val:expr, $next:expr, $default:expr) => {
        if let Some($ret_type) = $next{
            (2, $val)
        } else {
            (1, $default)
        }
    };
}

pub fn get_blocks(bytes: &Vec<u8>) -> HashMap<u16, InstructionBlock>{
    let mut indices: Vec<u16> = Vec::new();
    let mut code_to_instruction_map: HashMap<u16, Instruction> = HashMap::new();
    let mut pc = 0;

    while pc < bytes.len(){
        if let Ok((instruction, new_pc)) = parse_instruction(bytes, pc){
            code_to_instruction_map.insert(pc as u16, instruction);
            indices.push(pc as u16);
            pc = new_pc;
        }
    }
    let mut blocks = HashMap::new();
    let mut index_index = 0;
    let num_indices = indices.len();
    while index_index < num_indices{
        let index = indices[index_index];
        let next = indices.get(index_index+1).map(|i| code_to_instruction_map[i].clone());

        let instruction = code_to_instruction_map[&index].clone();
        let (offset, block) = match instruction{
            //AstoreWithoutPop
            Instruction::ASTORE(idx) => {
                if let Some(Instruction::ALOAD(idx2)) = next{
                    if idx == idx2
                    {(2, InstructionBlock::AStoreWithoutPop(idx as usize))} else 
                    {(1, InstructionBlock::Single(instruction))}
                } else {(1, InstructionBlock::Single(instruction))}
            }
            Instruction::ASTORE0 => store_without_pop!(Instruction::ALOAD0, InstructionBlock::AStoreWithoutPop(0), next, InstructionBlock::Single(instruction)),
            Instruction::ASTORE1 => store_without_pop!(Instruction::ALOAD1, InstructionBlock::AStoreWithoutPop(1), next, InstructionBlock::Single(instruction)),
            Instruction::ASTORE2 => store_without_pop!(Instruction::ALOAD2, InstructionBlock::AStoreWithoutPop(2), next, InstructionBlock::Single(instruction)),
            Instruction::ASTORE3 => store_without_pop!(Instruction::ALOAD3, InstructionBlock::AStoreWithoutPop(3), next, InstructionBlock::Single(instruction)),
            //Const Return
            Instruction::ICONST0 => const_ret!(Instruction::IRETURN, Value::Integer(0), next, InstructionBlock::Single(instruction)),
            Instruction::ICONST1 => const_ret!(Instruction::IRETURN, Value::Integer(1), next, InstructionBlock::Single(instruction)),
            Instruction::ICONST2 => const_ret!(Instruction::IRETURN, Value::Integer(2), next, InstructionBlock::Single(instruction)),
            Instruction::ICONST3 => const_ret!(Instruction::IRETURN, Value::Integer(3), next, InstructionBlock::Single(instruction)),
            Instruction::ICONST4 => const_ret!(Instruction::IRETURN, Value::Integer(4), next, InstructionBlock::Single(instruction)),
            Instruction::ICONST5 => const_ret!(Instruction::IRETURN, Value::Integer(5), next, InstructionBlock::Single(instruction)),
            Instruction::ICONSTM1 => const_ret!(Instruction::IRETURN, Value::Integer(-1), next, InstructionBlock::Single(instruction)),
            Instruction::LCONST0 => const_ret!(Instruction::LRETURN, Value::Long(0), next, InstructionBlock::Single(instruction)),
            Instruction::LCONST1 => const_ret!(Instruction::LRETURN, Value::Long(1), next, InstructionBlock::Single(instruction)),
            Instruction::FCONST0 => const_ret!(Instruction::FRETURN, Value::Float(0.0), next, InstructionBlock::Single(instruction)),
            Instruction::FCONST1 => const_ret!(Instruction::FRETURN, Value::Float(1.0), next, InstructionBlock::Single(instruction)),
            Instruction::FCONST2 => const_ret!(Instruction::FRETURN, Value::Float(2.0), next, InstructionBlock::Single(instruction)),
            Instruction::DCONST0 => const_ret!(Instruction::DRETURN, Value::Double(0.0), next, InstructionBlock::Single(instruction)),
            Instruction::DCONST1 => const_ret!(Instruction::DRETURN, Value::Double(1.0), next, InstructionBlock::Single(instruction)),
            Instruction::ACONST_NULL => const_ret!(Instruction::ARETURN, Value::Null, next, InstructionBlock::Single(instruction)),
            instruction => {(1, InstructionBlock::Single(instruction))}
        };
        blocks.insert(index, block);
        index_index += offset;
    }
    blocks
}