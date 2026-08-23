use crate::bytecode::Instruction;
use crate::vm::bytecode::IrInstruction;

pub type PC = u16;
pub type IrIndex = usize;

#[derive(Debug, Clone)]
pub struct LocatedInstruction {
    pub pc: PC,
    pub next_pc: PC,
    pub instruction: Instruction,
}

#[derive(Debug, Clone)]
pub struct LocatedIrInstruction {
    pub start_pc: PC,
    pub next_pc: PC,
    pub instruction: IrInstruction,
}

#[derive(Debug, Clone)]
pub struct IrCode {
    pub ir_instructions: Vec<LocatedIrInstruction>,
    pub pc_to_instruction_map: Vec<Option<IrIndex>>,
}

impl IrCode {
    pub fn get(&self, pc: PC) -> Option<&LocatedIrInstruction> {
        let index = self.pc_to_instruction_map.get(pc as usize)?.clone()?;
        self.ir_instructions.get(index)
    }
}