use crate::class_file::methods::attributes::{Code, ExceptionTableEntry};
use crate::class_file::methods::code::{IrCode, LocatedInstruction, LocatedIrInstruction};
use crate::vm::bytecode::decode;
use crate::{bytecode::Instruction, vm::bytecode::IrInstruction};
use std::collections::HashSet;

pub fn as_ir_code(code_attr: &Code) -> IrCode {
    let decoded = decode(&code_attr.code).unwrap();
    let ctx = build_optimization_context(&decoded, &code_attr.exception_table);
    let ir_instructions = optimize(&decoded, &ctx);
    let mut pc_to_instruction_map = vec![None; code_attr.code.len()];
    for (index, inst) in ir_instructions.iter().enumerate() {
        pc_to_instruction_map[inst.start_pc as usize] = Some(index);
    }
    IrCode {
        ir_instructions,
        pc_to_instruction_map,
    }
}

fn build_optimization_context(instructions: &[LocatedInstruction], exception_table: &[ExceptionTableEntry]) -> OptimizationContext {
    let mut barriers = HashSet::new();

    for inst in instructions {
        match &inst.instruction {
            Instruction::GOTO(t) | Instruction::IF_ACMPEQ(t) | Instruction::IF_ACMPNE(t) |
            Instruction::IF_ICMPEQ(t) | Instruction::IF_ICMPGE(t) | Instruction::IF_ICMPGT(t) |
            Instruction::IF_ICMPLE(t) | Instruction::IF_ICMPLT(t) | Instruction::IF_ICMPNE(t) |
            Instruction::IFEQ(t) | Instruction::IFNE(t) | Instruction::IFGT(t) |
            Instruction::IFLT(t) | Instruction::IFGE(t) | Instruction::IFLE(t) |
            Instruction::IFNULL(t) | Instruction::IFNONNULL(t) => {
                barriers.insert(*t);
            }
            Instruction::TABLESWITCH(_, _, default, targets) => {
                barriers.insert(*default);
                for target in targets {
                    barriers.insert(*target);
                }
            }
            Instruction::LOOKUPSWITCH(default, targets) => {
                barriers.insert(*default);
                for (_, target) in targets {
                    barriers.insert(*target);
                }
            }
            _ => {}
        }
    }

    for entry in exception_table {
        barriers.insert(entry.start_pc);
        barriers.insert(entry.end_pc);
        barriers.insert(entry.handler_pc);
    }

    OptimizationContext {
        barriers
    }
}

fn optimize(instructions: &[LocatedInstruction], ctx: &OptimizationContext) -> Vec<LocatedIrInstruction> {
    let mut result = Vec::with_capacity(instructions.len());

    let mut i = 0;

    while i < instructions.len() {
        if let Some((ir, consumed)) = try_optimize(&instructions[i..], ctx) {
            result.push(ir);
            i += consumed;
        } else {
            let inst = &instructions[i];

            result.push(LocatedIrInstruction {
                start_pc: inst.pc,
                next_pc: inst.next_pc,
                instruction: IrInstruction::Single(inst.instruction.clone()),
            });

            i += 1;
        }
    }

    result
}

fn try_optimize(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    try_o1_optimizations(input, ctx)
        .or_else(|| try_o2_optimizations(input, ctx))
}

pub struct OptimizationContext {
    pub barriers: HashSet<u16>,
}

fn try_o1_optimizations(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    try_store_load(input, ctx)
        .or_else(|| try_const_return(input, ctx))
}

#[cfg(feature = "o2")]
fn try_o2_optimizations(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    try_object_instantiation(input, ctx)
}

#[cfg(not(feature = "o2"))]
fn try_o2_optimizations(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    None
}

fn try_store_load(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    if input.len() < 2 {
        return None;
    }

    let first = &input[0];
    let second = &input[1];

    // The second instruction must not be independently
    // observable/referenced.
    if ctx.barriers.contains(&second.pc) {
        return None;
    }

    match (&first.instruction, &second.instruction) {
        (Instruction::ASTORE(str), Instruction::ALOAD(ld)) => {
            if str != ld { return None }
            Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::AStoreWithoutPop(*str as usize), }, 2))
        }
        (Instruction::ISTORE(str), Instruction::ILOAD(ld)) => {
            if str != ld { return None }
            Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::IStoreWithoutPop(*str as usize), }, 2))
        }
        (Instruction::LSTORE(str), Instruction::LLOAD(ld)) => {
            if str != ld { return None }
            Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::LStoreWithoutPop(*str as usize), }, 2))
        }
        (Instruction::FSTORE(str), Instruction::FLOAD(ld)) => {
            if str != ld { return None }
            Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::FStoreWithoutPop(*str as usize), }, 2))
        }
        (Instruction::DSTORE(str), Instruction::DLOAD(ld)) => {
            if str != ld { return None }
            Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::DStoreWithoutPop(*str as usize), }, 2))
        }

        // ...
        _ => None,
    }
}

fn try_const_return(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    if input.len() < 2 {
        return None;
    }

    let first = &input[0];
    let second = &input[1];

    // The second instruction must not be independently
    // observable/referenced.
    if ctx.barriers.contains(&second.pc) {
        return None;
    }

    match (&first.instruction, &second.instruction) {
        (Instruction::ICONST(amt), Instruction::IRETURN) => Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::IConstReturn(*amt), }, 2)),
        (Instruction::LCONST(amt), Instruction::LRETURN) => Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::LConstReturn(*amt), }, 2)),
        (Instruction::FCONST(amt), Instruction::FRETURN) => Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::FConstReturn(*amt), }, 2)),
        (Instruction::DCONST(amt), Instruction::DRETURN) => Some((LocatedIrInstruction { start_pc: first.pc, next_pc: second.next_pc, instruction: IrInstruction::DConstReturn(*amt), }, 2)),

        // ...
        _ => None,
    }
}

fn try_object_instantiation(input: &[LocatedInstruction], ctx: &OptimizationContext) -> Option<(LocatedIrInstruction, usize)> {
    if input.len() < 3 {
        return None;
    }

    let op_new = &input[0];
    let op_dup = &input[1];
    let op_constructor = &input[2];

    if ctx.barriers.contains(&op_dup.pc) || ctx.barriers.contains(&op_constructor.pc) {
        return None;
    }

    match (&op_new.instruction, &op_dup.instruction, &op_constructor.instruction) {
        (Instruction::NEW(class_idx), Instruction::DUP, Instruction::INVOKESPECIAL(method_idx)) => Some((
            LocatedIrInstruction { start_pc: op_new.pc, next_pc: op_constructor.next_pc, instruction: IrInstruction::ObjectInstantiation(*class_idx, *method_idx) },
            3,
        )),
        _ => None,
    }
}