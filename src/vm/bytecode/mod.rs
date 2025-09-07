use std::collections::BTreeMap;

use crate::{bytecode::Instruction, vm::value::Value};

mod il;
mod raw;

#[derive(Debug, PartialEq, Clone)]
pub enum InstructionBlock{
    Single(Instruction),
    AStoreWithoutPop(usize),
    IStoreWithoutPop(usize),
    LStoreWithoutPop(usize),
    FStoreWithoutPop(usize),
    DStoreWithoutPop(usize),
    IConstReturn(i32),
    LConstReturn(i64),
    FConstReturn(f32),
    DConstReturn(f64),
    JumpLabel,
    Jump(usize, Instruction)
}

const USE_RAW: bool = false;

pub fn get_blocks(bytes: &Vec<u8>) -> BTreeMap<u16, InstructionBlock>{
    if USE_RAW{
        raw::get_blocks(bytes).into_iter().enumerate().map(|(i, b)| (i as u16, b)).collect()
    } else {
        il::get_blocks(bytes)
    }
}

#[cfg(test)]
mod tests{
    use crate::{bytecode::Instruction, vm::{bytecode::{il, raw, InstructionBlock}, class_manager::ClassManager, class_path::ClassPath, value::Value}};

    #[test]
    fn test_raw(){
        let mut cp = ClassPath::default();
        cp.push("resources;resources/rt.jar").unwrap();
        let mut cl = ClassManager::new(cp);
        let clazz = cl.get_or_resolve_class("Slot").unwrap().get_class();

        let bytes = clazz.find_method("containsKey", "(Ljava/lang/Comparable;)Z").unwrap().code.clone().unwrap().code;
        let blocks = raw::get_blocks(&bytes);

        let expected = vec![
            Instruction::ALOAD0, 
            Instruction::GETFIELD(19), 
            Instruction::INVOKEVIRTUAL(36), 
            Instruction::ASTORE2,
            Instruction::ALOAD2,
            Instruction::INVOKEINTERFACE(40, 1, 0),
            Instruction::IFEQ(19),
            Instruction::ALOAD2,
            Instruction::INVOKEINTERFACE(46, 1, 0),
            Instruction::CHECKCAST(27),
            Instruction::ASTORE3,
            Instruction::ALOAD3,
            Instruction::GETFIELD(50),
            Instruction::ALOAD1,
            Instruction::INVOKEINTERFACE(54, 2, 0),
            Instruction::IFNE(18),
            Instruction::ICONST1,
            Instruction::IRETURN,
            Instruction::GOTO(4),
            Instruction::ICONST0,
            Instruction::IRETURN,
        ];
        for (index, expected_instruction) in expected.iter().enumerate(){
            if let InstructionBlock::Single(instruction) = &blocks[index]{
                assert_eq!(expected_instruction, instruction, "Instruction does not match. Expected {:?}, but found {:?}", expected_instruction, instruction);
            } else {
                assert!(false, "not a raw block");
            }
        }
    }
    
    #[test]
    fn test_il(){
        let mut cp = ClassPath::default();
        cp.push("resources;resources/rt.jar").unwrap();
        let mut cl = ClassManager::new(cp);
        let clazz = cl.get_or_resolve_class("Slot").unwrap().get_class();

        let bytes = clazz.find_method("containsKey", "(Ljava/lang/Comparable;)Z").unwrap().code.clone().unwrap().code;
        let blocks = il::get_blocks(&bytes);

        let expected = vec![
            InstructionBlock::Single(Instruction::ALOAD0), 
            InstructionBlock::Single(Instruction::GETFIELD(19)), 
            InstructionBlock::Single(Instruction::INVOKEVIRTUAL(36)),
            InstructionBlock::Single(Instruction::ASTORE2),
            InstructionBlock::Single(Instruction::ALOAD2),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(40, 1, 0)),
            InstructionBlock::Single(Instruction::IFEQ(45)),
            InstructionBlock::Single(Instruction::ALOAD2),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(46, 1, 0)),
            InstructionBlock::Single(Instruction::CHECKCAST(27)),
            InstructionBlock::AStoreWithoutPop(3),
            InstructionBlock::Single(Instruction::GETFIELD(50)),
            InstructionBlock::Single(Instruction::ALOAD1),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(54, 2, 0)),
            InstructionBlock::Single(Instruction::IFNE(42)),
            InstructionBlock::IConstReturn(1),
            InstructionBlock::Single(Instruction::GOTO(8)),
            InstructionBlock::IConstReturn(0),
        ];
        println!("{:#?}", blocks);
        assert_eq!(expected.len(), blocks.len());
        let mut actual_block_iter = blocks.values();
        let mut expected_block_iter = expected.iter();
        while let (Some(expected), Some(actual)) = (expected_block_iter.next(), actual_block_iter.next()){
            assert_eq!(expected, actual, "Instruction does not match. Expected {:?}, but found {:?}", expected, actual);
        }
    }
}