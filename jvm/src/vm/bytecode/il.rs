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
    let mut indices: Vec<u16> = Vec::new();
    let mut code_to_instruction_map: HashMap<u16, Instruction> = HashMap::new();
    let mut labels: Vec<u16> = Vec::new();
    let mut pc = 0;

    while pc < bytes.len(){
        if let Ok((instruction, new_pc)) = parse_instruction(bytes, pc){
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
            code_to_instruction_map.insert(pc as u16, instruction);
            indices.push(pc as u16);
            pc = new_pc;
        }
    }
    let mut blocks = BTreeMap::new();
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
        let end_index = if index_index + offset < num_indices{indices[index_index + offset]} else {indices[num_indices-1]};
        if labels.iter().any(|&l| l > index && l <= end_index){
            blocks.insert(index, InstructionBlock::Single(code_to_instruction_map[&index].clone()));
            index_index += 1;
        } else {
            blocks.insert(index, block);
            index_index += offset;
        }
    }
    blocks
}