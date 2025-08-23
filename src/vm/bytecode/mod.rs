use crate::{bytecode::Instruction, vm::value::Value};

mod il;
mod raw;

#[derive(Debug, PartialEq, Clone)]
pub enum InstructionBlock<'a>{
    Single(Instruction),
    AStoreWithoutPop(usize),
    IStoreWithoutPop(usize),
    LStoreWithoutPop(usize),
    FStoreWithoutPop(usize),
    DStoreWithoutPop(usize),
    ConstReturn(Value<'a>),
    JumpLabel,
    Jump(usize, Instruction)
}

pub fn get_blocks(bytes: &Vec<u8>){
    let raw = raw::get_blocks(bytes);
    let il = il::get_blocks(bytes);
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
            InstructionBlock::AStoreWithoutPop(2),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(40, 1, 0)),
            InstructionBlock::Single(Instruction::IFEQ(19)),
            InstructionBlock::Single(Instruction::ALOAD2),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(46, 1, 0)),
            InstructionBlock::Single(Instruction::CHECKCAST(27)),
            InstructionBlock::AStoreWithoutPop(3),
            InstructionBlock::Single(Instruction::GETFIELD(50)),
            InstructionBlock::Single(Instruction::ALOAD1),
            InstructionBlock::Single(Instruction::INVOKEINTERFACE(54, 2, 0)),
            InstructionBlock::Single(Instruction::IFNE(18)),
            InstructionBlock::ConstReturn(Value::Integer(1)),
            InstructionBlock::Single(Instruction::GOTO(4)),
            InstructionBlock::ConstReturn(Value::Integer(0)),
        ];
        for (index, expected_block) in expected.iter().enumerate(){
            assert_eq!(expected_block, &blocks[index], "Instruction does not match. Expected {:?}, but found {:?}", expected_block, blocks[index]);
        }
    }
}