use crate::bytecode::{parse_instruction, Instruction};
use crate::class_file::methods::code::LocatedInstruction;
use crate::vm::result::VMResult;

mod il;
mod raw;

#[derive(Debug, PartialEq, Clone)]
pub enum IrInstruction {
    // not optimized
    Single(Instruction),

    // optimization level 1
    AStoreWithoutPop(usize),
    IStoreWithoutPop(usize),
    LStoreWithoutPop(usize),
    FStoreWithoutPop(usize),
    DStoreWithoutPop(usize),
    IConstReturn(i32),
    LConstReturn(i64),
    FConstReturn(f32),
    DConstReturn(f64),

    // optimization level 2
    ObjectInstantiation(u16, u16),
}

fn decode(code: &[u8]) -> VMResult<Vec<LocatedInstruction>> {
    let mut pc = 0;
    let mut result = Vec::new();

    while pc < code.len() {
        let start_pc = pc;

        let (instruction, next_pc) = parse_instruction(code, pc)?;

        result.push(LocatedInstruction {
            pc: start_pc as u16,
            next_pc: next_pc as u16,
            instruction,
        });

        pc = next_pc;
    }

    Ok(result)
}

#[cfg(feature = "o1")]
pub use il::as_ir_code as as_ir_code;

#[cfg(not(feature = "o1"))]
pub use raw::as_ir_code as as_ir_code;
/*
#[cfg(test)]
mod tests{
    use crate::vm::VM;
    use crate::{bytecode::Instruction, vm::{bytecode::{il, raw, InstructionBlock}, class_path::ClassPath}};

    #[test]
    fn test_raw(){
        let mut cp = ClassPath::default();
        cp.push("../resources;../resources/rt.jar").unwrap();
        let vm = VM::new(cp);
        let clazz = vm.get_or_resolve_class("Slot").unwrap();

        let bytes = clazz.find_method("containsKey", "(Ljava/lang/Comparable;)Z").unwrap().attributes.code.clone().unwrap().code;
        let blocks = raw::get_blocks(&bytes);

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
            InstructionBlock::Single(Instruction::ASTORE3),
            InstructionBlock::Single(Instruction::ALOAD3),
            InstructionBlock::Single(Instruction::GETFIELD(50)),
            InstructionBlock::Single(Instruction::ALOAD1),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(54, 2, 0)),
            InstructionBlock::Single(Instruction::IFNE(42)),
            InstructionBlock::Single(Instruction::ICONST1),
            InstructionBlock::Single(Instruction::IRETURN),
            InstructionBlock::Single(Instruction::GOTO(8)),
            InstructionBlock::Single(Instruction::ICONST0),
            InstructionBlock::Single(Instruction::IRETURN),
        ];
        println!("{:#?}", blocks);
        assert_eq!(expected.len(), blocks.len());
        let mut actual_block_iter = blocks.values();
        let mut expected_block_iter = expected.iter();
        while let (Some(expected), Some(actual)) = (expected_block_iter.next(), actual_block_iter.next()){
            assert_eq!(expected, actual, "Instruction does not match. Expected {:?}, but found {:?}", expected, actual);
        }
    }
    
    #[test]
    fn test_il(){
        let mut cp = ClassPath::default();
        cp.push("../resources;../resources/rt.jar").unwrap();
        let vm = VM::new(cp);
        let clazz = vm.get_or_resolve_class("Slot").unwrap();

        let bytes = clazz.find_method("containsKey", "(Ljava/lang/Comparable;)Z").unwrap().attributes.code.clone().unwrap().code;
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
}*/