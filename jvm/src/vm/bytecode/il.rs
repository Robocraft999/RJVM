use std::collections::{BTreeMap, HashMap};

use crate::{bytecode::{parse_instruction, Instruction}, vm::bytecode::InstructionBlock};

macro_rules! const_ret {
    ($ret_type:pat, $val:expr, $next:expr, $default:expr) => {
        if let Some($ret_type) = $next{
            (2, $val)
        } else {
            (1, $default)
        }
    };
}

macro_rules! iconst_ret {
    ($val:expr, $next:expr, $default:expr) => {
        const_ret!(Instruction::IRETURN, InstructionBlock::IConstReturn($val), $next, $default)
    };
}

macro_rules! lconst_ret {
    ($val:expr, $next:expr, $default:expr) => {
        const_ret!(Instruction::LRETURN, InstructionBlock::LConstReturn($val), $next, $default)
    };
}

macro_rules! fconst_ret {
    ($val:expr, $next:expr, $default:expr) => {
        const_ret!(Instruction::FRETURN, InstructionBlock::FConstReturn($val), $next, $default)
    };
}

macro_rules! dconst_ret {
    ($val:expr, $next:expr, $default:expr) => {
        const_ret!(Instruction::DRETURN, InstructionBlock::DConstReturn($val), $next, $default)
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

pub fn get_blocks(bytes: &Vec<u8>) -> BTreeMap<u16, InstructionBlock>{
    let mut pcs: Vec<u16> = Vec::new();
    let mut pc_to_instruction_map: HashMap<u16, Instruction> = HashMap::new();
    let mut labels: Vec<u16> = Vec::new();
    let mut parse_pc = 0;

    while parse_pc < bytes.len(){
        if let Ok((instruction, new_parse_pc)) = parse_instruction(bytes, parse_pc){
            match instruction{
                Instruction::GOTO(t) | Instruction::IF_ACMPEQ(t) | Instruction::IF_ACMPNE(t) | 
                Instruction::IF_ICMPEQ(t) | Instruction::IF_ICMPGE(t) | Instruction::IF_ICMPGT(t) |
                Instruction::IF_ICMPLE(t) | Instruction::IF_ICMPLT(t) | Instruction::IF_ICMPNE(t) |
                Instruction::IFEQ(t) | Instruction::IFNE(t) | Instruction::IFGT(t) |
                Instruction::IFLT(t) | Instruction::IFGE(t) | Instruction::IFLE(t) |
                Instruction::IFNULL(t) | Instruction::IFNONNULL(t)
                => {labels.push(t);}
                _ => {}
            }
            pc_to_instruction_map.insert(parse_pc as u16, instruction);
            pcs.push(parse_pc as u16);
            parse_pc = new_parse_pc;
        }
    }
    let mut blocks = BTreeMap::new();
    let mut instruction_index = 0;
    let num_instructions = pcs.len();
    while instruction_index < num_instructions {
        let pc = pcs[instruction_index];
        let next = pcs.get(instruction_index +1).map(|i| pc_to_instruction_map[i].clone());

        let instruction = pc_to_instruction_map[&pc].clone();
        let (instruction_offset, block) = match instruction{
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
            Instruction::ICONST0 => iconst_ret!(0, next, InstructionBlock::Single(instruction)),
            Instruction::ICONST1 => iconst_ret!(1, next, InstructionBlock::Single(instruction)),
            Instruction::ICONST2 => iconst_ret!(2, next, InstructionBlock::Single(instruction)),
            Instruction::ICONST3 => iconst_ret!(3, next, InstructionBlock::Single(instruction)),
            Instruction::ICONST4 => iconst_ret!(4, next, InstructionBlock::Single(instruction)),
            Instruction::ICONST5 => iconst_ret!(5, next, InstructionBlock::Single(instruction)),
            Instruction::ICONSTM1 => iconst_ret!(-1, next, InstructionBlock::Single(instruction)),
            Instruction::LCONST0 => lconst_ret!(0, next, InstructionBlock::Single(instruction)),
            Instruction::LCONST1 => lconst_ret!(1, next, InstructionBlock::Single(instruction)),
            Instruction::FCONST0 => fconst_ret!(0.0, next, InstructionBlock::Single(instruction)),
            Instruction::FCONST1 => fconst_ret!(1.0, next, InstructionBlock::Single(instruction)),
            Instruction::FCONST2 => fconst_ret!(2.0, next, InstructionBlock::Single(instruction)),
            Instruction::DCONST0 => dconst_ret!(0.0, next, InstructionBlock::Single(instruction)),
            Instruction::DCONST1 => dconst_ret!(1.0, next, InstructionBlock::Single(instruction)),
            //Instruction::ACONST_NULL => const_ret!(Instruction::ARETURN, Value::Null, next, InstructionBlock::Single(instruction)),
            instruction => {(1, InstructionBlock::Single(instruction))}
        };
        let end_pc = if instruction_index + instruction_offset < num_instructions { pcs[instruction_index + instruction_offset]} else { u16::MAX };
        if labels.iter().any(|&label_pc| label_pc > pc && label_pc < end_pc){
            blocks.insert(pc, InstructionBlock::Single(pc_to_instruction_map[&pc].clone()));
            instruction_index += 1;
        } else {
            blocks.insert(pc, block);
            instruction_index += instruction_offset;
        }
    }
    blocks
}