use crate::class_file::methods::attributes::Code;
use crate::class_file::methods::code::{IrCode, LocatedIrInstruction};
use crate::vm::bytecode::decode;
use crate::vm::bytecode::IrInstruction;

pub fn as_ir_code(code_attr: &Code) -> IrCode {
    let decoded = decode(&code_attr.code).unwrap();

    let ir_instructions = decoded
        .into_iter()
        .map(|inst| LocatedIrInstruction {
            start_pc: inst.pc,
            next_pc: inst.next_pc,
            instruction: IrInstruction::Single(inst.instruction),
        })
        .collect::<Vec<_>>();

    let mut pc_to_instruction_map = vec![None; code_attr.code.len()];
    for (index, inst) in ir_instructions.iter().enumerate() {
        pc_to_instruction_map[inst.start_pc as usize] = Some(index);
    }
    IrCode {
        ir_instructions,
        pc_to_instruction_map,
    }
}