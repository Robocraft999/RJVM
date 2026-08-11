use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::class_manager::ClassLoadingState;
use crate::vm::constants::{MEMBERNAME_clazz_INDEX, MEMBERNAME_name_INDEX, MEMBERNAME_type_INDEX, THROWABLE_detailsMessage_INDEX};
use crate::vm::debug::validation::FieldTypeExt;
use crate::vm::java_thread::JavaThread;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::Context;
use crate::{bytecode::Instruction, get_or_init, get_or_init_option, vm::{bytecode::InstructionBlock, class::{ClassAndMethod, ClassRef}, java_error::JavaError, result::VMResult, value::{ReferenceType, Value}, VmError, VM}};
use log::{debug, error, info, trace, warn};
use parking_lot::RwLock;
use std::{str::FromStr};
use crate::vm::constants::classes::{JAVA_LANG_ARITHMETIC_EXCEPTION, JAVA_LANG_OBJECT};
use crate::vm::monitoring::MonitorAssociate;

macro_rules! wrap_error {
    ($res:expr) => {
        match $res{
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub fn execute<'a>(ctx: Context<'a, '_>) -> VMPartialResult<Option<Value>>{
    let camid = &ctx.thread.call_stack.get_class_and_method_id_cloned();
    let class_and_method = ClassAndMethod::try_resolve(ctx.vm, camid)?;
    if ctx.vm.class_manager.expect_class_state(class_and_method.class.id, ClassLoadingState::LOADED){
        unreachable!("Class {} has to be initialized to call {} upon", class_and_method.class.name, class_and_method.format());
    }
    info!("");
    info!("METHOD_NAME: {} at {}", class_and_method.format(), ctx.thread.call_stack.get_pc().0);
    trace!("{:?}", class_and_method.method.code_blocks);
    if let Some(_) = &class_and_method.method.attributes.code{
        let mut result = execute_current_block(ctx);
        while let None = result{
            result = execute_current_block(ctx);
        }
        return result.unwrap();
    }
    Err(VmError::MethodCallError(format!("Method: {} is not executeable, because it has no code", class_and_method.format())))
}

pub fn execute_current_block<'a>(ctx: Context<'a, '_>) -> Option<VMPartialResult<Option<Value>>>{
    let camid = &ctx.thread.call_stack.get_class_and_method_id_cloned();
    let class_and_method = wrap_error!(ClassAndMethod::try_resolve(ctx.vm, camid));
    let block = class_and_method.method.get_code_block_at(ctx.thread.call_stack.get_pc());
    let current_pc = ctx.thread.call_stack.get_pc();
    trace!(">{:03} {:?}", current_pc.0, block);
    trace!("stack[{}]=", class_and_method.get_max_stack_size());
    for (index, value) in ctx.thread.call_stack.operand_stacks.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    trace!("locals[{}]=", class_and_method.get_max_locals());
    for (index, value) in ctx.thread.call_stack.locals_stack.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    if let Some(next_pc) = class_and_method.method.next_pc(current_pc){
        ctx.thread.call_stack.set_pc(next_pc);
    }
    
    match block{
        InstructionBlock::Single(instruction) => {
            match instruction{
                Instruction::ACONST_NULL => x_const(ctx.thread, ctx.vm.null()),
                Instruction::ICONSTM1 => x_const(ctx.thread, Value::Integer(-1)),
                Instruction::ICONST0 => x_const(ctx.thread, Value::Integer(0)),
                Instruction::ICONST1 => x_const(ctx.thread, Value::Integer(1)),
                Instruction::ICONST2 => x_const(ctx.thread, Value::Integer(2)),
                Instruction::ICONST3 => x_const(ctx.thread, Value::Integer(3)),
                Instruction::ICONST4 => x_const(ctx.thread, Value::Integer(4)),
                Instruction::ICONST5 => x_const(ctx.thread, Value::Integer(5)),
                Instruction::LCONST0 => x_const(ctx.thread, Value::Long(0)),
                Instruction::LCONST1 => x_const(ctx.thread, Value::Long(1)),
                Instruction::FCONST0 => x_const(ctx.thread, Value::Float(0.0)),
                Instruction::FCONST1 => x_const(ctx.thread, Value::Float(1.0)),
                Instruction::FCONST2 => x_const(ctx.thread, Value::Float(2.0)),
                Instruction::DCONST0 => x_const(ctx.thread, Value::Double(0.0)),
                Instruction::DCONST1 => x_const(ctx.thread, Value::Double(1.0)),
                Instruction::BIPUSH(value) => {
                    debug!("BIPUSH {:?}", value);
                    ctx.thread.call_stack.push_operand_value(Value::Integer(*value as i32))
                }
                Instruction::SIPUSH(value) => {
                    debug!("SIPUSH {:?}", value);
                    ctx.thread.call_stack.push_operand_value(Value::Integer(*value as i32))
                }

                Instruction::LDC(index) => {
                    let value = get_or_init_option!(get_constant_as_value(ctx, (*index) as u16));
                    debug!("LDC: {}", value.print(ctx.vm));
                    ctx.thread.call_stack.push_operand_value(value);
                }
                Instruction::LDCW(index) => {
                    let value = get_or_init_option!(get_constant_as_value(ctx, *index));
                    debug!("LDCW: {}", value.print(ctx.vm));
                    ctx.thread.call_stack.push_operand_value(value);
                }
                Instruction::LDC2W(index) => {
                    let value = get_or_init_option!(get_constant_as_value(ctx, *index));
                    debug!("LDC2W: {}", value.print(ctx.vm));
                    ctx.thread.call_stack.push_operand_value(value);
                }

                Instruction::ILOAD(index) => wrap_error!(iload(ctx.thread, *index as usize)),
                Instruction::LLOAD(index) => wrap_error!(lload(ctx.thread, *index as usize)),
                Instruction::FLOAD(index) => wrap_error!(fload(ctx.thread, *index as usize)),
                Instruction::DLOAD(index) => wrap_error!(dload(ctx.thread, *index as usize)),
                Instruction::ALOAD(index) => wrap_error!(aload(ctx.thread, *index as usize)),

                Instruction::ILOAD0 => wrap_error!(iload(ctx.thread, 0)),
                Instruction::ILOAD1 => wrap_error!(iload(ctx.thread, 1)),
                Instruction::ILOAD2 => wrap_error!(iload(ctx.thread, 2)),
                Instruction::ILOAD3 => wrap_error!(iload(ctx.thread, 3)),

                Instruction::LLOAD0 => wrap_error!(lload(ctx.thread, 0)),
                Instruction::LLOAD1 => wrap_error!(lload(ctx.thread, 1)),
                Instruction::LLOAD2 => wrap_error!(lload(ctx.thread, 2)),
                Instruction::LLOAD3 => wrap_error!(lload(ctx.thread, 3)),

                Instruction::FLOAD0 => wrap_error!(fload(ctx.thread, 0)),
                Instruction::FLOAD1 => wrap_error!(fload(ctx.thread, 1)),
                Instruction::FLOAD2 => wrap_error!(fload(ctx.thread, 2)),
                Instruction::FLOAD3 => wrap_error!(fload(ctx.thread, 3)),

                Instruction::DLOAD0 => wrap_error!(dload(ctx.thread, 0)),
                Instruction::DLOAD1 => wrap_error!(dload(ctx.thread, 1)),
                Instruction::DLOAD2 => wrap_error!(dload(ctx.thread, 2)),
                Instruction::DLOAD3 => wrap_error!(dload(ctx.thread, 3)),

                Instruction::ALOAD0 => wrap_error!(aload(ctx.thread, 0)),
                Instruction::ALOAD1 => wrap_error!(aload(ctx.thread, 1)),
                Instruction::ALOAD2 => wrap_error!(aload(ctx.thread, 2)),
                Instruction::ALOAD3 => wrap_error!(aload(ctx.thread, 3)),

                Instruction::IALOAD | Instruction::LALOAD | Instruction::FALOAD | Instruction::DALOAD | Instruction::AALOAD | Instruction::BALOAD | Instruction::CALOAD | Instruction::SALOAD => {
                    let index = ctx.thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    let array = ctx.thread.call_stack.pop_operand_value();
                    debug!("XALOAD: {:?}[{}]", array, index);
                    if let Some(Value::Reference(array_id)) = array{
                        let array_ref = wrap_error!(ctx.vm.resolve_object_by_id(array_id));
                        ctx.thread.call_stack.push_operand_value(array_ref.get_element(index as usize));
                    }
                }

                Instruction::ISTORE(index) => wrap_error!(istore(ctx.thread, *index as usize)),
                Instruction::LSTORE(index) => wrap_error!(lstore(ctx.thread, *index as usize)),
                Instruction::FSTORE(index) => wrap_error!(fstore(ctx.thread, *index as usize)),
                Instruction::DSTORE(index) => wrap_error!(dstore(ctx.thread, *index as usize)),
                Instruction::ASTORE(index) => wrap_error!(astore(ctx.thread, *index as usize)),

                Instruction::ISTORE0 => wrap_error!(istore(ctx.thread, 0)),
                Instruction::ISTORE1 => wrap_error!(istore(ctx.thread, 1)),
                Instruction::ISTORE2 => wrap_error!(istore(ctx.thread, 2)),
                Instruction::ISTORE3 => wrap_error!(istore(ctx.thread, 3)),

                Instruction::LSTORE0 => wrap_error!(lstore(ctx.thread, 0)),
                Instruction::LSTORE1 => wrap_error!(lstore(ctx.thread, 1)),
                Instruction::LSTORE2 => wrap_error!(lstore(ctx.thread, 2)),
                Instruction::LSTORE3 => wrap_error!(lstore(ctx.thread, 3)),

                Instruction::FSTORE0 => wrap_error!(fstore(ctx.thread, 0)),
                Instruction::FSTORE1 => wrap_error!(fstore(ctx.thread, 1)),
                Instruction::FSTORE2 => wrap_error!(fstore(ctx.thread, 2)),
                Instruction::FSTORE3 => wrap_error!(fstore(ctx.thread, 3)),

                Instruction::DSTORE0 => wrap_error!(dstore(ctx.thread, 0)),
                Instruction::DSTORE1 => wrap_error!(dstore(ctx.thread, 1)),
                Instruction::DSTORE2 => wrap_error!(dstore(ctx.thread, 2)),
                Instruction::DSTORE3 => wrap_error!(dstore(ctx.thread, 3)),

                Instruction::ASTORE0 => wrap_error!(astore(ctx.thread, 0)),
                Instruction::ASTORE1 => wrap_error!(astore(ctx.thread, 1)),
                Instruction::ASTORE2 => wrap_error!(astore(ctx.thread, 2)),
                Instruction::ASTORE3 => wrap_error!(astore(ctx.thread, 3)),

                Instruction::IASTORE | Instruction::LASTORE | Instruction::FASTORE | Instruction::DASTORE | Instruction::AASTORE | Instruction::BASTORE | Instruction::CASTORE | Instruction::SASTORE => {
                    //TODO validate type of value to fit instruction
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let index = ctx.thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    let popped = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("XASTORE: {:?}[{}] <- {:?}", popped, index, value);
                    if let Value::Reference(array_id) = popped{
                        let array_ref = wrap_error!(ctx.vm.resolve_object_by_id(array_id));
                        #[cfg(feature = "debug")]
                        ctx.thread.debug_helper.tracker.push_object_event(array_id, format!("Setting [{}] to:\n    {:?}", index, array_ref.print(ctx.vm)));
                        array_ref.set_element(index as usize, value);
                    }
                }

                Instruction::POP => {
                    debug!("POP");
                    if ctx.thread.call_stack.pop_operand_value().is_none(){
                        return Some(Err(VmError::ValidationError("Expected a value to pop but Stack was empty".to_owned())));
                    }
                }
                Instruction::POP2 => {
                    debug!("POP2");
                    let popped1 = ctx.thread.call_stack.pop_operand_value();
                    if let Some(val) = popped1{
                        if val.get_computational_type() == 1{
                            if ctx.thread.call_stack.pop_operand_value().is_none(){
                                return Some(Err(VmError::ValidationError("Expected a second value to pop but Stack was empty".to_owned())));
                            }
                        }
                    } else {
                        return Some(Err(VmError::ValidationError("Expected a value to pop but Stack was empty".to_owned())));
                    }
                }
                Instruction::DUP => {
                    debug!("DUP");
                    let top = ctx.thread.call_stack.pop_operand_value().unwrap();
                    ctx.thread.call_stack.push_operand_value(top.clone());
                    ctx.thread.call_stack.push_operand_value(top);
                }
                Instruction::DUPX1 => {
                    debug!("DUPX1");
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    ctx.thread.call_stack.push_operand_value(value.clone());
                    ctx.thread.call_stack.push_operand_value(value2);
                    ctx.thread.call_stack.push_operand_value(value);
                }
                Instruction::DUPX2 => {
                    debug!("DUPX2");
                    let value1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if value2.get_computational_type() == 1{
                        let value3 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value3);
                        ctx.thread.call_stack.push_operand_value(value2);
                        ctx.thread.call_stack.push_operand_value(value1);
                    } else {
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value2);
                        ctx.thread.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2 => {
                    debug!("DUP2");
                    let value1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1{
                        let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        ctx.thread.call_stack.push_operand_value(value2.clone());
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value2);
                        ctx.thread.call_stack.push_operand_value(value1);
                    } else {
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2X1 => {
                    debug!("DUP2X1");
                    let value1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1{
                        let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        let value3 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        ctx.thread.call_stack.push_operand_value(value2.clone());
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value3);
                        ctx.thread.call_stack.push_operand_value(value2);
                        ctx.thread.call_stack.push_operand_value(value1);
                    } else {
                        let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        ctx.thread.call_stack.push_operand_value(value1.clone());
                        ctx.thread.call_stack.push_operand_value(value2);
                        ctx.thread.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2X2 => {
                    debug!("DUP2X1");
                    let value1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 2{
                        if value2.get_computational_type() == 2{
                            ctx.thread.call_stack.push_operand_value(value1.clone());
                            ctx.thread.call_stack.push_operand_value(value2);
                            ctx.thread.call_stack.push_operand_value(value1);
                        } else {
                            let value3 = ctx.thread.call_stack.pop_operand_value().unwrap();
                            ctx.thread.call_stack.push_operand_value(value1.clone());
                            ctx.thread.call_stack.push_operand_value(value3);
                            ctx.thread.call_stack.push_operand_value(value2);
                            ctx.thread.call_stack.push_operand_value(value1);
                        }
                    } else {
                        let value3 = ctx.thread.call_stack.pop_operand_value().unwrap();
                        if value3.get_computational_type() == 2{
                            ctx.thread.call_stack.push_operand_value(value2.clone());
                            ctx.thread.call_stack.push_operand_value(value1.clone());
                            ctx.thread.call_stack.push_operand_value(value3);
                            ctx.thread.call_stack.push_operand_value(value2);
                            ctx.thread.call_stack.push_operand_value(value1);
                        } else {
                            let value4 = ctx.thread.call_stack.pop_operand_value().unwrap();
                            ctx.thread.call_stack.push_operand_value(value2.clone());
                            ctx.thread.call_stack.push_operand_value(value1.clone());
                            ctx.thread.call_stack.push_operand_value(value4);
                            ctx.thread.call_stack.push_operand_value(value3);
                            ctx.thread.call_stack.push_operand_value(value2);
                            ctx.thread.call_stack.push_operand_value(value1);
                        }
                    }
                }
                Instruction::SWAP => {
                    debug!("SWAP");
                    let value1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let value2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1 && value2.get_computational_type() == 1{
                        ctx.thread.call_stack.push_operand_value(value1);
                        ctx.thread.call_stack.push_operand_value(value2);
                    } else {
                        return Some(Err(VmError::ValidationError("SWAP can only be applied to computational type 1 values".to_owned())));
                    }
                }

                Instruction::IADD => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_add(val2)))),
                Instruction::LADD => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_add(val2)))),
                Instruction::FADD => wrap_error!(execute_f_arithmetic(ctx.thread, |val1, val2| Ok(val1 + val2))),
                Instruction::DADD => wrap_error!(execute_d_arithmetic(ctx.thread, |val1, val2| Ok(val1 + val2))),

                Instruction::ISUB => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_sub(val2)))),
                Instruction::LSUB => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_sub(val2)))),
                Instruction::FSUB => wrap_error!(execute_f_arithmetic(ctx.thread, |val1, val2| Ok(val1 - val2))),
                Instruction::DSUB => wrap_error!(execute_d_arithmetic(ctx.thread, |val1, val2| Ok(val1 - val2))),

                Instruction::IMUL => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::LMUL => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::FMUL => wrap_error!(execute_f_arithmetic(ctx.thread, |val1, val2| Ok(val1 * val2))),
                Instruction::DMUL => wrap_error!(execute_d_arithmetic(ctx.thread, |val1, val2| Ok(val1 * val2))),

                Instruction::IDIV => {
                    let value2 = ctx.thread.call_stack.pop_operand_value();
                    let value1 = ctx.thread.call_stack.pop_operand_value();
                    if let (Some(Value::Integer(val1)), Some(Value::Integer(val2))) = (value1, value2){
                        if val2 == 0 {
                            let error_clazz = get_or_init_option!(ctx.get_or_initialize_class(JAVA_LANG_ARITHMETIC_EXCEPTION));
                            return Some(JavaThread::throw(ctx, error_clazz, "Division by Zero".to_owned(), class_and_method.format()))
                        }
                        let res = val1.wrapping_div(val2);
                        debug!("Integer ARITHMETIC {}/{}={}", val1, val2, res);
                        ctx.thread.call_stack.push_operand_value(Value::Integer(res));
                    } else {
                        return Some(Err(VmError::ValidationError("Expected two ints".to_string())))
                    }
                }
                Instruction::LDIV => {
                    let value2 = ctx.thread.call_stack.pop_operand_value();
                    let value1 = ctx.thread.call_stack.pop_operand_value();
                    if let (Some(Value::Long(val1)), Some(Value::Long(val2))) = (value1, value2){
                        if val2 == 0 {
                            let error_clazz = get_or_init_option!(ctx.get_or_initialize_class(JAVA_LANG_ARITHMETIC_EXCEPTION));
                            return Some(JavaThread::throw(ctx, error_clazz, "Division by Zero".to_owned(), class_and_method.format()))
                        }
                        let res = val1.wrapping_div(val2);
                        debug!("Long ARITHMETIC {}/{}={}", val1, val2, res);
                        ctx.thread.call_stack.push_operand_value(Value::Long(res));
                    } else {
                        return Some(Err(VmError::ValidationError("Expected two longs".to_string())))
                    }
                }
                Instruction::FDIV => wrap_error!(execute_f_arithmetic(ctx.thread, |val1, val2| Ok(val1 / val2))),
                Instruction::DDIV => wrap_error!(execute_d_arithmetic(ctx.thread, |val1, val2| Ok(val1 / val2))),

                Instruction::IREM => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_rem(val2)))),
                Instruction::LREM => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1.wrapping_rem(val2)))),

                Instruction::INEG => {
                    let value = wrap_error!(ctx.thread.call_stack.pop_operand_value().unwrap().expect_int());
                    ctx.thread.call_stack.push_operand_value(Value::Integer(-value))
                }
                Instruction::LNEG => {
                    let value = wrap_error!(ctx.thread.call_stack.pop_operand_value().unwrap().expect_long());
                    ctx.thread.call_stack.push_operand_value(Value::Long(-value))
                }
                Instruction::FNEG => {
                    let value = wrap_error!(ctx.thread.call_stack.pop_operand_value().unwrap().expect_float());
                    ctx.thread.call_stack.push_operand_value(Value::Float(-value))
                }

                Instruction::ISHL => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1 << (val2 & 0x1f)))),
                Instruction::LSHL => wrap_error!(execute_ji_arithmetic(ctx.thread, |val1, val2| Ok(val1 << (val2 & 0x3f)))),
                Instruction::ISHR => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1 >> (val2 & 0x1f)))),
                Instruction::LSHR => wrap_error!(execute_ji_arithmetic(ctx.thread, |val1, val2| Ok(val1 >> (val2 & 0x3f)))),
                Instruction::IUSHR => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| {
                    if val1 > 0{
                        Ok(val1 >> (val2 & 0x1f))
                    } else {
                        Ok(((val1 as u32) >> (val2 & 0x1f)) as i32)
                    }
                })),
                Instruction::LUSHR => wrap_error!(execute_ji_arithmetic(ctx.thread, |val1, val2| {
                    if val1 > 0{
                        Ok(val1 >> (val2 & 0x1f))
                    } else {
                        Ok(((val1 as u64) >> (val2 & 0x1f)) as i64)
                    }
                })),

                Instruction::IAND => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1 & val2))),
                Instruction::LAND => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1 & val2))),
                Instruction::IOR  => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1 | val2))),
                Instruction::LOR  => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1 | val2))),
                Instruction::IXOR => wrap_error!(execute_i_arithmetic(ctx.thread, |val1, val2| Ok(val1 ^ val2))),
                Instruction::LXOR => wrap_error!(execute_l_arithmetic(ctx.thread, |val1, val2| Ok(val1 ^ val2))),
                Instruction::IINC(index, amount) => {
                    if let Some(Value::Integer(value)) = ctx.thread.call_stack.load_local(*index as usize){
                        ctx.thread.call_stack.store_local(Value::Integer(value + *amount as i32), *index as usize);
                    }
                }

                //TODO fix conversions to work always
                Instruction::I2L => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2L");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Long(val as i64));
                    } else {
                        warn!("I2L Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2F => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2F");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Float(val as f32));
                    } else {
                        warn!("I2F Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2D => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2D");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Double(val as f64));
                    } else {
                        warn!("I2D Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::L2I => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("L2I");
                    if let Value::Long(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("L2I Conversion failed, because {value:?} is not of type Long")
                    }
                }
                Instruction::L2F => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("L2F");
                    if let Value::Long(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Float(val as f32));
                    } else {
                        warn!("L2F Conversion failed, because {value:?} is not of type Long")
                    }
                }
                Instruction::F2I => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("F2I");
                    if let Value::Float(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("F2I Conversion failed, because {value:?} is not of type Float")
                    }
                }
                Instruction::F2D => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("F2D");
                    if let Value::Float(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Double(val as f64));
                    } else {
                        warn!("F2D Conversion failed, because {value:?} is not of type Float")
                    }
                }
                Instruction::D2I => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("D2I");
                    if let Value::Double(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("D2I Conversion failed, because {value:?} is not of type Double")
                    }
                }
                Instruction::D2L => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("D2L");
                    if let Value::Double(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Long(val as i64));
                    } else {
                        warn!("D2L Conversion failed, because {value:?} is not of type Double")
                    }
                }
                Instruction::D2F => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("D2F");
                    if let Value::Double(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Float(val as f32));
                    } else {
                        warn!("D2F Conversion failed, because {value:?} is not of type Double")
                    }
                }
                Instruction::I2B => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2B");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer((val as u8) as i32));
                    } else {
                        warn!("I2B Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2C => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2C");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer((val as u16) as i32));
                    } else {
                        warn!("I2C Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2S => {
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("I2S");
                    if let Value::Integer(val) = value {
                        ctx.thread.call_stack.push_operand_value(Value::Integer((val as i16) as i32));
                    } else {
                        warn!("I2S Conversion failed, because {value:?} is not of type Integer")
                    }
                }

                Instruction::LCMP => {
                    if let (Some(Value::Long(value2)), Some(Value::Long(value1))) = (ctx.thread.call_stack.pop_operand_value(), ctx.thread.call_stack.pop_operand_value()) {
                        debug!("LCMP");
                        if value1 > value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(-1))
                        }
                    }
                }
                Instruction::FCMPG | Instruction::FCMPL => {
                    if let (Some(Value::Float(value2)), Some(Value::Float(value1))) = (ctx.thread.call_stack.pop_operand_value(), ctx.thread.call_stack.pop_operand_value()) {
                        debug!("FCMP");
                        if value1 > value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(-1))
                        } else if value1.is_nan() || value2.is_nan() {
                            if instruction == &Instruction::FCMPG {
                                ctx.thread.call_stack.push_operand_value(Value::Integer(1))
                            } else {
                                ctx.thread.call_stack.push_operand_value(Value::Integer(-1))
                            }
                        }
                    }
                }
                Instruction::DCMPG | Instruction::DCMPL => {
                    if let (Some(Value::Double(value2)), Some(Value::Double(value1))) = (ctx.thread.call_stack.pop_operand_value(), ctx.thread.call_stack.pop_operand_value()) {
                        debug!("DCMP");
                        if value1 > value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            ctx.thread.call_stack.push_operand_value(Value::Integer(-1))
                        } else if value1.is_nan() || value2.is_nan() {
                            if instruction == &Instruction::DCMPG {
                                ctx.thread.call_stack.push_operand_value(Value::Integer(1))
                            } else {
                                ctx.thread.call_stack.push_operand_value(Value::Integer(-1))
                            }
                        }
                    }
                }

                Instruction::IFEQ(target) => { execute_cmp(ctx.thread, *target, |value| value == 0) }
                Instruction::IFNE(target) => { execute_cmp(ctx.thread, *target, |value| value != 0) }
                Instruction::IFLT(target) => { execute_cmp(ctx.thread, *target, |value| value <  0) }
                Instruction::IFGE(target) => { execute_cmp(ctx.thread, *target, |value| value >= 0) }
                Instruction::IFGT(target) => { execute_cmp(ctx.thread, *target, |value| value >  0) }
                Instruction::IFLE(target) => { execute_cmp(ctx.thread, *target, |value| value <= 0) }

                Instruction::IF_ICMPNE(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 != val2),
                Instruction::IF_ICMPGT(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 >  val2),
                Instruction::IF_ICMPGE(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 >= val2),
                Instruction::IF_ICMPEQ(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 == val2),
                Instruction::IF_ICMPLT(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 <  val2),
                Instruction::IF_ICMPLE(target) => execute_i_cmp(ctx.thread, *target, |val1, val2| val1 <= val2),

                Instruction::IF_ACMPEQ(target) => {
                    let o1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let o2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    match (o1, o2) {
                        (Value::Reference(obj1), Value::Reference(obj2)) => {
                            debug!("IF_ACMPEQ {:?} == {:?}?", obj1, obj2);
                            if obj1 == obj2 {
                                ctx.thread.call_stack.set_pc(*target);
                            }
                        }
                        _ => {}
                    };
                }
                Instruction::IF_ACMPNE(target) => {
                    let o1 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let o2 = ctx.thread.call_stack.pop_operand_value().unwrap();
                    match (o1, o2) {
                        (Value::Reference(obj1), Value::Reference(obj2)) => {
                            debug!("IF_ACMPNE {:?} != {:?}?", obj1, obj2);
                            if obj1 != obj2 {
                                ctx.thread.call_stack.set_pc(*target);
                            }
                        }
                        _ => {}
                    };
                }

                Instruction::GOTO(target) => ctx.thread.call_stack.set_pc(*target),

                Instruction::TABLESWITCH(default, low, high, offsets) => {
                    let index = ctx.thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    if index < *low || index > *high{
                        debug!("TABLESWITCH default {}", default);
                        ctx.thread.call_stack.set_pc((current_pc.0 as i32 + default) as u16);
                    } else {
                        let offset = offsets[(index - low) as usize];
                        debug!("TABLESWITCH[{}]: {}", index, offset);
                        ctx.thread.call_stack.set_pc((current_pc.0 as i32 + offset) as u16);
                    }
                }
                Instruction::LOOKUPSWITCH(default, pair_stream) => {
                    let popped = ctx.thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    debug!("LOOKUPSWITCH: {}", popped);
                    let mut use_default = true;
                    for chunk in pair_stream.chunks(2){
                        let (int_match, target) = (chunk[0], chunk[1]);
                        if int_match == popped{
                            ctx.thread.call_stack.set_pc(target as u16);
                            use_default = false;
                            break;
                        }
                    }
                    if use_default{
                        ctx.thread.call_stack.set_pc(*default as u16);
                    }
                }

                Instruction::IRETURN | Instruction::LRETURN | Instruction::FRETURN | Instruction::DRETURN | Instruction::ARETURN => {
                    //TODO seperate for validation
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    debug!("XRETURN: {}", value.print(ctx.vm));
                    if !class_and_method.method.is_static(){
                        if let Some(Value::Reference(this)) = ctx.thread.call_stack.load_local(0){
                            #[cfg(feature = "debug")]
                            ctx.thread.debug_helper.tracker.push_object_event(this, format!("Function {} returned:\n    {}", class_and_method.format(), value.print(ctx.vm)))
                        }
                    }
                    #[cfg(feature = "debug")]
                    ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), format!("returning: {} at {}", value.print(ctx.vm), ctx.thread.call_stack.get_pc().0));
                    if !class_and_method.method.descriptor.return_type.clone().map(|rt| rt == value).unwrap_or(false) {
                        unreachable!("Trying to return {:?} but expecting: {:?}", value, class_and_method.method.descriptor.return_type)
                    }
                    #[cfg(feature = "validation")]
                    {
                        wrap_error!(class_and_method.method.descriptor.return_type.clone().unwrap().validate(value, ctx));
                    }
                    return Some(Ok(VMResultType::Successful(Some(value))))
                }
                Instruction::RETURN => {
                    debug!("RETURN");
                    #[cfg(feature = "debug")]
                    ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), "returning".to_owned());
                    if class_and_method.method.name == "<clinit>"{
                        ctx.vm.class_manager.update_class_state(class_and_method.class, ClassLoadingState::INITIALIZED);
                    }
                    return Some(Ok(VMResultType::Successful(None)))
                }

                Instruction::PUTSTATIC(index) => {
                    let caf = class_and_method.get_constant_field_ref(&ctx, *index).unwrap();
                    get_or_init_option!(ctx.ensure_initialized(caf.class));
                    if ctx.vm.class_manager.expect_class_state(caf.class.id, ClassLoadingState::LOADED){
                        unimplemented!()
                    }
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();

                    let (field_index, info, class_id) = caf.class.find_field_static(caf.field.name.as_str()).unwrap();
                    get_or_init_option!(ctx.ensure_initialized(ctx.vm.find_class_by_id(class_id).unwrap()));
                    let object = ctx.vm.get_static_class_object(class_id).unwrap();
                    #[cfg(feature = "debug")]
                    ctx.thread.debug_helper.tracker.push_object_event(object.id, format!("Set static field: {}: {:?} to:\n    {}", info.name, info.field_type, value.print(ctx.vm)));
                    debug!("PUTSTATIC {} {} {} {:?}", caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    #[cfg(feature = "validation")]
                    {
                        wrap_error!(info.field_type.validate(value, ctx));
                    }
                    object.set_field(field_index, value);
                }
                Instruction::GETSTATIC(index) => {
                    let caf = class_and_method.get_constant_field_ref(&ctx, *index).unwrap();
                    get_or_init_option!(ctx.ensure_initialized(caf.class));
                    if ctx.vm.class_manager.expect_class_state(caf.class.id, ClassLoadingState::LOADED){
                        unimplemented!()
                    }
                    let (field_index, info, class_id) = caf.class.find_field_static(caf.field.name.as_str()).unwrap();
                    get_or_init_option!(ctx.ensure_initialized(ctx.vm.find_class_by_id(class_id).unwrap()));
                    let object = ctx.vm.get_static_class_object(class_id).unwrap();
                    debug!("GETSTATIC {} {} {} {:?}", caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    let value = object.get_field(field_index);
                    #[cfg(feature = "validation")]
                    {
                        wrap_error!(info.field_type.validate(value, ctx));
                    }
                    ctx.thread.call_stack.push_operand_value(value);
                }
                Instruction::GETFIELD(index) => {
                    let caf = class_and_method.get_constant_field_ref(&ctx, *index).unwrap();
                    debug!("GETFIELD {}.{} {}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor());
                    let (field_index, info) = caf.class.find_field(caf.field.name.as_str()).unwrap();
                    let object = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj_id) = object && !object.is_null(){
                        let obj = wrap_error!(ctx.vm.resolve_object_by_id(obj_id));
                        let value = obj.get_field(field_index);
                        #[cfg(feature = "validation")]
                        {
                            wrap_error!(info.field_type.validate(value, ctx));
                        }
                        ctx.thread.call_stack.push_operand_value(value);
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Cannot get field: {}.{}::{} because 'this' is {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), object))));
                    }
                }
                Instruction::PUTFIELD(index) => {
                    let caf = class_and_method.get_constant_field_ref(&ctx, *index).unwrap();
                    let (field_index, info) = caf.class.find_field(caf.field.name.as_str()).unwrap();
                    debug!("PUTFIELD {}.{} {} {} {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    let value = ctx.thread.call_stack.pop_operand_value().unwrap();
                    let object = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj_id) = object && !object.is_null(){
                        let obj = wrap_error!(ctx.vm.resolve_object_by_id(obj_id));
                        #[cfg(feature = "debug")]
                        ctx.thread.debug_helper.tracker.push_object_event(obj_id, format!("Set field: {}: {:?} to:\n    {}", info.name, info.field_type, value.print(ctx.vm)));
                        #[cfg(feature = "validation")]
                        {
                            wrap_error!(info.field_type.validate(value, ctx));
                        }
                        obj.set_field(field_index, value);
                        debug!("obj:{:?}", obj.print(ctx.vm));
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Cannot get field: {}.{}::{} because 'this' is {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), object))));
                    }
                }

                Instruction::INVOKEVIRTUAL(index) => { return Some(execute_invoke(ctx, *index, InvokeKind::VIRTUAL)) }
                Instruction::INVOKESPECIAL(index) => { return Some(execute_invoke(ctx, *index, InvokeKind::SPECIAL)) }
                Instruction::INVOKESTATIC(index) => { return Some(execute_invoke(ctx, *index, InvokeKind::STATIC)) }
                Instruction::INVOKEINTERFACE(index, _, _) => { return Some(execute_invoke(ctx, *index, InvokeKind::INTERFACE)) }
                Instruction::INVOKEDYNAMIC(index, _, _) => {
                    debug!("INVOKEDYNAMIC");
                    let Some(ConstantPoolEntry::InvokeDynamic(bm, name, typ)) = class_and_method.class.get_or_resolve_constant(&ctx, *index) else { unreachable!("Do Errors") };
                    let caller_obj = Value::Reference(get_or_init_option!(ctx.new_class_object_by_class(class_and_method.class)).id);

                    let Some(ConstantPoolEntry::MethodHandleMethod(bm_kind, bootstrap_cam)) = class_and_method.class.get_or_resolve_constant(&ctx, bm.bootstrap_method_ref) else { unreachable!("Do Errors") };
                    let Some(Value::Reference(bm_type_id)) = get_or_init_option!(ctx.new_method_type(&bootstrap_cam.method.descriptor)) else { unreachable!("Do errors") };
                    let bm_type_ref = wrap_error!(ctx.vm.resolve_object_by_id(bm_type_id));
                    let bootstrap_method_obj = get_or_init_option!(ctx.new_method_handle(class_and_method.class, bm_kind, bootstrap_cam, bm_type_ref)).unwrap();

                    let name_obj = Value::Reference(get_or_init_option!(ctx.new_string_object(name.as_str())).id);
                    let Some(type_obj) = get_or_init_option!(ctx.new_method_type(&MethodDescriptor::new(typ))) else { unreachable!("Do Errors") };

                    let mut static_args = Vec::new();
                    for index in bm.bootstrap_arguments.iter() {
                        let Some(val) = (match class_and_method.class.get_or_resolve_constant(&ctx, *index) {
                            Some(ConstantPoolEntry::MethodType(desc)) => get_or_init_option!(ctx.new_method_type(&desc)),
                            Some(ConstantPoolEntry::MethodHandleMethod(arg_kind, arg_cam)) => {
                                let Some(Value::Reference(arg_method_type_id)) = get_or_init_option!(ctx.new_method_type(&arg_cam.method.descriptor)) else {
                                    return Some(Err(VmError::ValidationError("Could not create MethodType for static callsite arg".to_owned())));
                                };
                                let arg_method_type = wrap_error!(ctx.vm.resolve_object_by_id(arg_method_type_id));
                                get_or_init_option!(ctx.new_method_handle(class_and_method.class, arg_kind, arg_cam, arg_method_type))
                            }
                            _ => unimplemented!()
                        }) else {
                            return Some(Err(VmError::ValidationError("Could not load static arg for invokedynamic".to_owned())));
                        };
                        static_args.push(val);
                    }
                    let static_arguments = Value::Reference(get_or_init_option!(ctx.new_object_array_1(static_args)).id);

                    let appendix_ref = get_or_init_option!(ctx.new_object_array_1(vec![ctx.vm.null()]));
                    let appendix_result = Value::Reference(appendix_ref.id);

                    let helper = ctx.resolve_class_method(
                        "java/lang/invoke/MethodHandleNatives",
                        "linkCallSite",
                        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/invoke/MemberName;"
                    ).unwrap();
                    
                    let Some(Value::Reference(mname_id)) = get_or_init_option!(JavaThread::invoke_subroutine(ctx, helper, None, vec![caller_obj, bootstrap_method_obj, name_obj, type_obj, static_arguments, appendix_result])) else { unreachable!("DO ERRORs") };
                    let mname_ref = wrap_error!(ctx.vm.resolve_object_by_id(mname_id));

                    let Value::Reference(typ_id) = mname_ref.get_field(MEMBERNAME_type_INDEX) else { unreachable!("DO errors") };
                    let typ_ref = wrap_error!(ctx.vm.resolve_object_by_id(typ_id));

                    let Value::Reference(class_ref_id) = mname_ref.get_field(MEMBERNAME_clazz_INDEX) else { unreachable!("Do Errors") };
                    let clazz = wrap_error!(ctx.resolve_clazz_by_class_ref_id(class_ref_id));
                    let name = ctx.vm.extract_string_from_value(mname_ref.get_field(MEMBERNAME_name_INDEX)).unwrap();

                    let desc = ctx.extract_descriptor_from_method_type(typ_ref).unwrap();
                    let desc = MethodDescriptor::new(desc);

                    let method = clazz.find_method(name.as_str(), desc.as_str()).unwrap();
                    let cam = ClassAndMethod { class: clazz, method};

                    let mut args = vec![];
                    for _ in 0..cam.method.get_args_count()-1 {
                        let popped = ctx.thread.call_stack.pop_operand_value().unwrap();
                        match popped {
                            Value::Long(_) | Value::Double(_) => {args.insert(0, Value::Dummy)}
                            _ => {}
                        }
                        args.insert(0, popped);
                    }
                    args.push(appendix_ref.get_element(0));
                    if let Some(res) = get_or_init_option!(JavaThread::invoke_subroutine(ctx, cam, None, args)) {
                        ctx.thread.call_stack.push_operand_value(res);
                    }
                }

                Instruction::NEW(index) => {
                    let clazz = class_and_method.get_constant_class_ref(&ctx, *index).unwrap();
                    get_or_init_option!(ctx.ensure_initialized(clazz));
                    if ctx.vm.class_manager.expect_class_state(clazz.id, ClassLoadingState::LOADED){
                        unimplemented!("Cannot create instance of {:?} if not initializ-ed/-ing", clazz.name);
                    }
                    let new_object = ctx.new_object_from_class(clazz);

                    debug!("NEW: {} {} {:?}", index, clazz.name, &new_object.print(ctx.vm));
                    ctx.thread.call_stack.push_operand_value(Value::Reference(new_object.id));
                }
                Instruction::NEWARRAY(atype) => {
                    let primitive_type = match atype {
                        4  => FieldType::Primitive(PrimitiveType::Boolean),
                        5  => FieldType::Primitive(PrimitiveType::Char),
                        6  => FieldType::Primitive(PrimitiveType::Float),
                        7  => FieldType::Primitive(PrimitiveType::Double),
                        8  => FieldType::Primitive(PrimitiveType::Byte),
                        9  => FieldType::Primitive(PrimitiveType::Short),
                        10 => FieldType::Primitive(PrimitiveType::Integer),
                        11 => FieldType::Primitive(PrimitiveType::Long),
                        _ => unreachable!("Can not create an array of type {atype}")
                    };
                    let array_field_type = primitive_type.to_array_field_type(1);
                    let array = get_or_init_option!(execute_create_array(ctx, array_field_type, 1));

                    debug!("NEWARRAY {}", atype);
                    ctx.thread.call_stack.push_operand_value(array);
                }
                Instruction::ANEWARRAY(index) => {
                    let class = class_and_method.get_constant_class_ref(&ctx, *index).unwrap();
                    let array_field_type = FieldType::Object(class.name.clone()).to_array_field_type(1);
                    let array = get_or_init_option!(execute_create_array(ctx, array_field_type, 1));
                    
                    debug!("ANEWARRAY {}", class.name);
                    ctx.thread.call_stack.push_operand_value(array);
                }
                Instruction::ARRAYLENGTH => {
                    debug!("ARRAYLENGTH");
                    let popped = ctx.thread.call_stack.pop_operand_value();
                    if let Some(Value::Reference(ref_id)) = popped{
                        let reference = wrap_error!(ctx.vm.resolve_object_by_id(ref_id));
                        ctx.thread.call_stack.push_operand_value(Value::Integer(reference.get_length() as i32));
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Expected an array ref but found: {:?}", &popped))))
                    }
                }

                Instruction::ATHROW => {
                    debug!("ATHROW");
                    if let Some(Value::Reference(error_id)) = ctx.thread.call_stack.pop_operand_value(){
                        let error = wrap_error!(ctx.vm.resolve_object_by_id(error_id));
                        let string_value = error.get_field(THROWABLE_detailsMessage_INDEX);
                        let string = if !string_value.is_null() {
                            match ctx.vm.extract_string_from_value(string_value) {
                                Ok(s) => s,
                                Err(VmError::CESU8Error(_)) => String::from("<CESU Decode Error>"),
                                Err(e) => return Some(Err(e))
                            }
                        } else {String::new()};
                        let exception_name = ctx.vm.class_manager.find_class_by_id(error.class_id).unwrap().name.clone();
                        #[cfg(feature = "debug")]
                        ctx.thread.debug_helper.exception_helper.push(format!("Throw   {}: {}\n└-- thrown by {} at {}", exception_name, string, class_and_method.format(), ctx.thread.call_stack.get_pc().0));
                        let prev = ctx.thread.caught_exception.replace(Some((string, class_and_method.format(), Value::Reference(error.id))));
                        assert!(prev.is_none());
                        return Some(Ok(VMResultType::ExceptionThrown));
                    }
                    return Some(Err(VmError::JavaException(JavaError::JavaExceptionThrown("JavaException".to_string(), "Unknown".to_string(), class_and_method.format()))));
                }

                Instruction::CHECKCAST(constant_index) => {
                    //TODO
                    debug!("CHECKCAST {:?}", &class_and_method.class.get_or_resolve_constant(&ctx, *constant_index));
                }
                Instruction::INSTANCEOF(constant_index) => {
                    let of_class = match class_and_method.class.get_or_resolve_constant(&ctx, *constant_index){
                        Some(ConstantPoolEntry::Class(class_ref)) => class_ref,
                        _ => return Some(Err(VmError::ValidationError("Expected a resolvable class entry".to_string()))),
                    };

                    let object = ctx.thread.call_stack.pop_operand_value().unwrap();
                    if object.is_null(){
                        ctx.thread.call_stack.push_operand_value(Value::from(false));
                        return None;
                    }
                    let Value::Reference(object_id) = object else { return Some(Err(VmError::ValidationError("INSTANCEOF: expected object to be a reference".to_string()))) };
                    let object = wrap_error!(ctx.vm.resolve_object_by_id(object_id));
                    let object_class = ctx.vm.find_class_by_id(object.class_id).unwrap();
                    let instance_of = ctx.vm.is_instance_of(object_class, of_class);

                    debug!("INSTANCEOF {:?} = {}", &class_and_method.class.get_or_resolve_constant(&ctx, *constant_index), instance_of);

                    ctx.thread.call_stack.push_operand_value(Value::from(instance_of));
                }

                Instruction::MONITORENTER => {
                    if let Some(Value::Reference(lock_ref)) = ctx.thread.call_stack.pop_operand_value(){
                        debug!("MONITORENTER");
                        if lock_ref.is_null() {
                            return Some(Err(VmError::ValidationError("Can not lock on null".to_string())))
                        }
                        #[cfg(feature = "debug")]
                        ctx.thread.debug_helper.monitor_logger.push_event(MonitorAssociate::Ref(lock_ref), format!("ENTER in {} at {}", class_and_method.format(), ctx.thread.call_stack.get_pc().0));
                        wrap_error!(ctx.vm.monitor_handler.enter_ref_or_block(ctx, lock_ref));
                    } else {
                        warn!("No object to lock")
                    }
                }
                Instruction::MONITOREXIT => {
                    if let Some(Value::Reference(lock_ref)) = ctx.thread.call_stack.pop_operand_value(){
                        debug!("MONITOREXIT");
                        if lock_ref.is_null() {
                            return Some(Err(VmError::ValidationError("Can not unlock on null".to_string())))
                        }
                        #[cfg(feature = "debug")]
                        ctx.thread.debug_helper.monitor_logger.push_event(MonitorAssociate::Ref(lock_ref), format!("EXIT in {} at {}", class_and_method.format(), ctx.thread.call_stack.get_pc().0));
                        wrap_error!(ctx.vm.monitor_handler.exit_ref(ctx, lock_ref))
                    } else {
                        warn!("No object to lock")
                    }
                }

                Instruction::WIDE(op, index, const_option) => {
                    match Instruction::from_repr(*op).unwrap(){
                        Instruction::IINC(..) => {
                            if let (Some(Value::Integer(value)), Some(amount)) = (ctx.thread.call_stack.load_local(*index as usize), const_option) {
                                ctx.thread.call_stack.store_local(Value::Integer(value + *amount as i32), *index as usize);
                            } else {
                                return Some(Err(VmError::ValidationError("Expected an int and a constant value".to_owned())))
                            }
                        }
                        unknown => unreachable!("WIDE with op: {:?} not executable", unknown)
                    }
                }
                Instruction::MULTIANEWARRAY(index, dimensions ) => {
                    if let Some(ConstantPoolEntry::Class(clazz)) = class_and_method.class.get_or_resolve_constant(&ctx, *index){
                        let class_name = clazz.name.as_str();
                        let array_field_type = FieldType::from_str(class_name).unwrap();
                        let array = get_or_init_option!(execute_create_array(ctx, array_field_type, *dimensions as usize));
                        debug!("MULTIANEWARRAY {}", class_name);
                        ctx.thread.call_stack.push_operand_value(array);
                    }
                }

                Instruction::IFNULL(target) => {
                    let reference = ctx.thread.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Reference(r) => {
                            if r.is_null(){
                                debug!("+IFNULL is NULL");
                                ctx.thread.call_stack.set_pc(*target);
                            } else {
                                debug!("-IFNULL is reference");
                            }
                        }
                        _ => {warn!("?IFNULL {:?} is this valid?", reference.clone())}
                    }
                }
                Instruction::IFNONNULL(target) => {
                    let reference = ctx.thread.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Reference(r) => {
                            if r.is_null(){
                                debug!("-IFNONNULL is NULL");
                            } else {
                                debug!("+IFNONNULL is reference");
                                ctx.thread.call_stack.set_pc(*target);
                            }
                        }
                        _ => {warn!("?IFNONNULL {:?} is this valid?", reference.clone())}
                    }
                }
                other => {
                    return Some(Err(VmError::Unspecified(format!("Single Instruction of type {:?} not executable", other))))
                }
            }
        }
        InstructionBlock::AStoreWithoutPop(index) => {
            let top = ctx.thread.call_stack.operand_stacks.borrow().last().unwrap().last().unwrap().clone();
            ctx.thread.call_stack.store_local(top, *index);
        }
        InstructionBlock::IConstReturn(val) => {
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), format!("returning int: {}", val));
            return Some(Ok(VMResultType::Successful(Some(Value::Integer(*val)))))
        }
        InstructionBlock::LConstReturn(val) => {
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), format!("returning long: {}", val));
            return Some(Ok(VMResultType::Successful(Some(Value::Long(*val)))))
        }
        other => {
            return Some(Err(VmError::Unspecified(format!("Block of type {:?} not executable", other))))
        }
    }
    debug!("");
    None
}

fn x_const<'a>(thread: &JavaThread, value: Value){
    debug!("XCONST: {:?}", value);
    thread.call_stack.push_operand_value(value);
}

fn istore(thread: &JavaThread, index: usize) -> VMResult<()> {
    let value = thread.call_stack.pop_operand_value().unwrap();
    debug!("ISTORE{} {:?}", index, value);
    thread.call_stack.store_local(value, index);
    Ok(())
}

//TODO validation
fn lstore(thread: &JavaThread, index: usize) -> VMResult<()> {
    let value = thread.call_stack.pop_operand_value().unwrap();
    debug!("LSTORE{} {:?}", index, value);
    thread.call_stack.store_local(value, index);
    thread.call_stack.store_local(Value::Dummy, index+1);
    Ok(())
}

fn fstore(thread: &JavaThread, index: usize) -> VMResult<()> {
    let value = thread.call_stack.pop_operand_value().unwrap();
    debug!("FSTORE{} {:?}", index, value);
    thread.call_stack.store_local(value, index);
    Ok(())
}

fn dstore(thread: &JavaThread, index: usize) -> VMResult<()> {
    let value = thread.call_stack.pop_operand_value().unwrap();
    debug!("DSTORE{} {:?}", index, value);
    thread.call_stack.store_local(value, index);
    thread.call_stack.store_local(Value::Dummy, index+1);
    Ok(())
}

fn astore(thread: &JavaThread, index: usize) -> VMResult<()> {
    let value = thread.call_stack.pop_operand_value().unwrap();
    debug!("ASTORE{} {:?}", index, value);
    thread.call_stack.store_local(value, index);
    Ok(())
}

fn iload(thread: &JavaThread, index: usize) -> VMResult<()> {
    let popped = thread.call_stack.load_local(index).unwrap();
    match popped {
        Value::Integer(i) => {
            debug!("ILOAD{} {}", index, i);
        }
        _ => return Err(VmError::ValidationError(format!("ILOAD{} failed", index)))
    }
    thread.call_stack.push_operand_value(popped);
    Ok(())
}

fn lload(thread: &JavaThread, index: usize) -> VMResult<()> {
    let local = thread.call_stack.load_local(index);
    let dummy = thread.call_stack.load_local(index + 1);
    if dummy.as_ref().unwrap() != &Value::Dummy{
        return Err(VmError::ValidationError(format!("Expected a Dummy value at {} but got {:?}",index+1, dummy.unwrap())));
    }
    if let Some(Value::Long(value)) = local{
        thread.call_stack.push_operand_value(Value::Long(value));
        debug!("LLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("LLOAD{} failed, because locals[{}] was {:?} and not Long", index, index, local)))
    }
}

fn fload(thread: &JavaThread, index: usize) -> VMResult<()> {
    let local = thread.call_stack.load_local(index);
    if let Some(Value::Float(value)) = local{
        thread.call_stack.push_operand_value(Value::Float(value));
        debug!("FLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("FLOAD{} failed, because locals[{}] was {:?} and not Float", index, index, local)))
    }
}

fn dload(thread: &JavaThread, index: usize) -> VMResult<()> {
    let local = thread.call_stack.load_local(index);
    if let Some(Value::Double(value)) = local{
        thread.call_stack.push_operand_value(Value::Double(value));
        debug!("DLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("DLOAD{} failed, because locals[{}] was {:?} and not Double", index, index, local)))
    }
}

fn aload<'a>(thread: &JavaThread, index: usize) -> VMResult<()>{
    let popped = thread.call_stack.load_local(index).unwrap();
    match popped {
        Value::Reference(reference) => {
            debug!("ALOAD{} {:?}", index, reference);
        }
        p => return Err(VmError::ValidationError(format!("ALOAD{} failed: got '{:?}'", index, p)))
    }
    thread.call_stack.push_operand_value(popped);
    Ok(())
}

fn execute_cmp<F: FnOnce(i32) -> bool>(thread: &JavaThread, target: u16, cmp: F){
    let value = thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    if cmp(value){
        thread.call_stack.set_pc(target);
    }
}

fn execute_i_cmp<F: FnOnce(i32, i32) -> bool>(thread: &JavaThread, offset: u16, f: F){
    let val2 = thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    let val1 = thread.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    let jump = f(val1, val2);
    debug!("ICMP: {}&{}={}", val1, val2, jump);
    if jump{
        thread.call_stack.set_pc(offset);
    }
}

fn execute_i_arithmetic<F: FnOnce(i32, i32) -> VMResult<i32>>(thread: &JavaThread, f: F) -> VMResult<()> {
    let value2 = thread.call_stack.pop_operand_value();
    let value1 = thread.call_stack.pop_operand_value();
    if let (Some(Value::Integer(val1)), Some(Value::Integer(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Integer ARITHMETIC {}&{}={}", val1, val2, res);
        thread.call_stack.push_operand_value(Value::Integer(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee ints to keck");
        Err(VmError::ValidationError("Expected two ints".to_string()))
    }
}

fn execute_l_arithmetic<F: FnOnce(i64, i64) -> VMResult<i64>>(thread: &JavaThread, f: F) -> VMResult<()> {
    let value2 = thread.call_stack.pop_operand_value();
    let value1 = thread.call_stack.pop_operand_value();
    if let (Some(Value::Long(val1)), Some(Value::Long(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Long ARITHMETIC {}&{}={}", val1, val2, res);
        thread.call_stack.push_operand_value(Value::Long(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee longs to keck");
        Err(VmError::ValidationError("Expected two longs".to_string()))
    }
}

fn execute_f_arithmetic<F: FnOnce(f32, f32) -> VMResult<f32>>(thread: &JavaThread, f: F) -> VMResult<()> {
    let value2 = thread.call_stack.pop_operand_value();
    let value1 = thread.call_stack.pop_operand_value();
    if let (Some(Value::Float(val1)), Some(Value::Float(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Float ARITHMETIC {}&{}={}", val1, val2, res);
        thread.call_stack.push_operand_value(Value::Float(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee floats to keck");
        Err(VmError::ValidationError("Expected two floats".to_string()))
    }
}

fn execute_d_arithmetic<F: FnOnce(f64, f64) -> VMResult<f64>>(thread: &JavaThread, f: F) -> VMResult<()> {
    let value2 = thread.call_stack.pop_operand_value();
    let value1 = thread.call_stack.pop_operand_value();
    if let (Some(Value::Double(val1)), Some(Value::Double(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Double ARITHMETIC {}&{}={}", val1, val2, res);
        thread.call_stack.push_operand_value(Value::Double(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee doubles to keck");
        Err(VmError::ValidationError("Expected two doubles".to_string()))
    }
}

fn execute_ji_arithmetic<F: FnOnce(i64, i32) -> Result<i64, VmError>>(thread: &JavaThread, f: F) -> VMResult<()> {
    let value2 = thread.call_stack.pop_operand_value();
    let value1 = thread.call_stack.pop_operand_value();
    if let (Some(Value::Long(val1)), Some(Value::Integer(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("LongInt ARITHMETIC {}&{}={}", val1, val2, res);
        thread.call_stack.push_operand_value(Value::Long(res));
        Ok(())
    } else {
        warn!("dat sin nich eene long und eene int du keck");
        Err(VmError::ValidationError("Expected an int and a long".to_string()))
    }
}

fn execute_invoke<'a>(ctx: Context<'a, '_>, index: u16, kind: InvokeKind) -> VMPartialResult<Option<Value>> {
    let calling_class_and_method_id = &ctx.thread.call_stack.get_class_and_method_id_cloned();
    let calling_class_and_method = &ClassAndMethod::try_resolve(ctx.vm, calling_class_and_method_id)?;

    let (cam, args_count) = get_constant_method_ref_and_args_count(calling_class_and_method, &ctx, index).expect("GIB MICH DIE METHODE");
    trace!("loading class to execute on: '{}'", cam.class.name.as_str());
    get_or_init!(ctx.ensure_initialized(cam.class)?);
    if ctx.vm.class_manager.expect_class_state(cam.class.id, ClassLoadingState::LOADED) {
        unimplemented!()
    }
    trace!("loading state is: {:?}", ctx.vm.class_manager.class_loading_states.read().get(&cam.class.id));
    trace!("finished loading class to execute on: '{}'", cam.class.name.as_str());
    trace!("args_count: {}", args_count);
    let mut args = Vec::new();
    for _ in 0..args_count{
        let popped = ctx.thread.call_stack.pop_operand_value().unwrap();
        match popped {
            Value::Long(_) | Value::Double(_) => {args.insert(0, Value::Dummy)}
            _ => {}
        }
        args.insert(0, popped);
    }

    #[cfg(feature = "validation")]
    if (kind == InvokeKind::STATIC && !cam.method.is_static()) || (kind != InvokeKind::STATIC && cam.method.is_static()){
        return Err(VmError::ValidationError(format!("[Validation]: kind is: {:?} but method is_static? {}", kind, cam.method.is_static())));
    }

    let class_and_method = match kind {
        InvokeKind::SPECIAL | InvokeKind::STATIC => {
            cam.class
                .find_method(cam.method.name.as_str(), cam.method.descriptor.as_str())
                .map(|method| ClassAndMethod {class: cam.class, method})
                .unwrap_or(get_method_virtual(cam.class, cam.method.name.as_str(), cam.method.descriptor.as_str())?)
        }
        InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
            get_method_virtual(cam.class, cam.method.name.as_str(), cam.method.descriptor.as_str())?
        }
    };
    let receiver = if class_and_method.method.is_static(){
        None
    } else {
        let popped = ctx.thread.call_stack.pop_operand_value();
        if let Some(Value::Reference(ref_id)) = popped && !ref_id.is_null(){
            let reference = ctx.vm.resolve_object_by_id(ref_id)?;
            Some(reference)
        } else {
            println!("XXXX: {} {:?}", class_and_method.class.name, ctx.vm.class_manager.class_loading_states.read().get(&class_and_method.class.id));
            return Err(VmError::ValidationError(format!("Expected object or array as receiver for {} but found: {:?}", class_and_method.format(), popped)));
        }
    };
    let class_and_method = match kind {
        InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
            match receiver {
                Some(obj) => {
                    let receiver_class = ctx.vm.find_class_by_id(obj.class_id).unwrap();
                    #[cfg(feature = "validation")]{
                        let is_instance = if class_and_method.class.name == JAVA_LANG_OBJECT || class_and_method.class.is_array(){
                            true
                        } else {
                            ctx.vm.is_instance_of(receiver_class, class_and_method.class)
                        };
                        if !is_instance{
                            ctx.vm.mark_canceled();
                            return Err(VmError::ValidationError(format!("[Validation]: Expected subclass of: {} but got: {}", class_and_method.class.name, obj.print(ctx.vm))));
                        }
                    }
                    let method_resolver = if kind == InvokeKind::VIRTUAL {get_method_virtual} else {get_method_interface_virtual};
                    let resolved_method = method_resolver(receiver_class, class_and_method.method.name.as_str(), class_and_method.method.descriptor.as_str())?;
                    resolved_method
                }
                None => {
                    error!("Receiver was not found");
                    class_and_method
                }
            }
        }
        _ => class_and_method
    };

    trace!("STATUS of '{}' before invoke: ", class_and_method.method.name);
    trace!("stack=");
    for (index, value) in ctx.thread.call_stack.operand_stacks.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    trace!("locals=");
    for (index, value) in ctx.thread.call_stack.locals_stack.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    debug!("INVOKE{:?}: {}{} on {:?}", kind, cam.method.name, cam.method.descriptor.as_str(), receiver);
    #[cfg(feature = "debug")]
    {
        if let Some(rec) = receiver{
            ctx.thread.debug_helper.tracker.push_object_event(rec.id, format!("Preparing call {} with args:{}", class_and_method.format(), args.iter().map(|v| format!("\n    {}", v.print(ctx.vm))).collect::<Vec<_>>().join("")));
            ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), format!("Calling on {} from {} with args: {}", rec.print(ctx.vm), calling_class_and_method.format(), args.iter().map(|v| format!("\n    {}", v.print(ctx.vm))).collect::<Vec<_>>().join("") ));
        } else {
            ctx.thread.debug_helper.tracker.push_method_event(class_and_method.format(), format!("Calling static from {} with args: {}", calling_class_and_method.format(), args.iter().map(|v| format!("\n    {}", v.print(ctx.vm))).collect::<Vec<_>>().join("") ));
        }
    }
    if !class_and_method.class.has_method_polymorphic_signature(class_and_method.method) {
        for (i, provided_arg) in args.iter().filter(|a| if let Value::Dummy = a {false} else {true}).enumerate(){
            if !(&class_and_method.method.descriptor.args[i] == provided_arg){
                return Err(VmError::ValidationError(format!("Expected arg type: {:?} but got value: {:?}", class_and_method.method.descriptor.args[i], provided_arg)));
            }
        }
    }
    ctx.create_and_push_call_frame(class_and_method, receiver, args, true);
    Ok(VMResultType::Interrupted(1, false))
    //Ok(VMResultType::Ok(Some(Value::Null)))
    /*let res = vm.invoke(class_and_method, receiver, args)?.to_option();
    if res.is_some(){
        self.stack.push(res.unwrap())
    }
    Ok(())*/
}

fn get_method_virtual<'a>(class: ClassRef<'a>, method_name: &str, descriptor: &str) -> Result<ClassAndMethod<'a>, VmError>{
    let mut current_class = class;
    if current_class.is_array() && method_name == "clone"{
        while let Some(super_class) = current_class.superclass{
            current_class = super_class;
        }
        return Ok(ClassAndMethod{class: current_class, method: current_class.find_method(method_name, descriptor).unwrap()})
    }
    if class.is_interface(){
        loop {
            if let Some(method) = current_class.find_method(method_name, descriptor){
                return Ok(ClassAndMethod{class: current_class, method});
            }
            if let Some(super_interface) = current_class.interfaces.first(){
                current_class = super_interface
            } else {
                return Err(VmError::JavaException(JavaError::MethodNotFoundException(format!("{}{} in {}", method_name, descriptor, class.name))));
            }
        }
    } else {
        loop {
            if let Some(method) = current_class.find_method(method_name, descriptor){
                return Ok(ClassAndMethod{class: current_class, method});
            }
            if let Some(super_class) = current_class.superclass{
                current_class = super_class
            } else {
                return Err(VmError::JavaException(JavaError::MethodNotFoundException(format!("{}{} in {}", method_name, descriptor, class.name))));
            }
        }
    }
}

fn get_method_interface_virtual<'a>(class: ClassRef<'a>, method_name: &str, descriptor: &str) -> Result<ClassAndMethod<'a>, VmError>{
    let mut current_class = class;
    loop {
        if let Some(method) = current_class.find_method(method_name, descriptor){
            return Ok(ClassAndMethod{class: current_class, method});
        }
        if let Some(super_class) = current_class.superclass{
            if super_class.superclass.is_some(){
                current_class = super_class
            } else {
                if let Some(super_interface) = current_class.interfaces.first(){
                    current_class = super_interface
                } else {
                    return Err(VmError::JavaException(JavaError::MethodNotFoundException(format!("{}{} in {}", method_name, descriptor, class.name))));
                }
            }
        } else {
            return Err(VmError::JavaException(JavaError::MethodNotFoundException(format!("{}{} in {}", method_name, descriptor, class.name))));
        }
    }
}

fn get_constant_method_ref_and_args_count<'a>(calling: &ClassAndMethod<'a>, ctx: &Context<'a, '_>, index: u16) -> Option<(ClassAndMethod<'a>, usize)> {
    match calling.class.get_or_resolve_constant(ctx, index) {
        Some(ConstantPoolEntry::MethodRef(cam)) | Some(ConstantPoolEntry::InterfaceMethodRef(cam)) => {
            let args_count = cam.method.descriptor.args.len();
            Some((cam, args_count))
        }
        Some(ConstantPoolEntry::MethodRefSigPoly(cam, desc)) => {
            Some((cam, desc.args.len()))
        }
        _ => None
    }
}

//FIXME: Deprecated
fn get_constant_as_value<'a>(ctx: Context<'a, '_>, index: u16) -> VMPartialResult<Value>{
    let camid = &ctx.thread.call_stack.get_class_and_method_id_cloned();
    let class_and_method = ClassAndMethod::try_resolve(ctx.vm, camid)?;
    let constant_value = class_and_method.class.get_or_resolve_constant(&ctx, index).unwrap();
    let value = match constant_value {
        ConstantPoolEntry::Integer(value) => Value::Integer(value),
        ConstantPoolEntry::Long(value) => Value::Long(value),
        ConstantPoolEntry::Float(value) => Value::Float(value),
        ConstantPoolEntry::Double(value) => Value::Double(value),
        ConstantPoolEntry::String(string) => {
            let string_object = get_or_init!(ctx.new_string_object(string.as_str())?);
            Value::Reference(string_object.id)
        }
        ConstantPoolEntry::Class(clazz) => {
            let class_object = get_or_init!(ctx.new_class_object_by_class(clazz)?);
            Value::Reference(class_object.id)
        }
        _ => unimplemented!("Constant of type {constant_value:?} cannot be converted to a value")
    };
    Ok(VMResultType::Successful(value))
}

fn execute_create_array<'a>(ctx: Context<'a, '_>, array_field_type: FieldType, dims: usize) -> VMPartialResult<Value>{
    if let FieldType::Array(_, component_type) = array_field_type{
        //ensure that the array class get loaded before popping the count(s)
        for i in 0..dims{
            let _ = ctx.get_or_resolve_class(component_type.clone().to_array_field_type(i+1).to_class_name().as_str())?;
        }
        let mut content = Vec::new();
        for i in 0..dims{
            let current_dim = ctx.thread.call_stack.pop_operand_value().unwrap().expect_int()?;
            if current_dim == 0{
                break;
            }
            let mut local_content = Vec::new();
            if i == 0{
                local_content = vec![component_type.get_default_value(ctx.vm.null()); current_dim as usize];
                content = local_content;
                continue
            }
            for _ in 0..current_dim{
                let arr_ref = ctx.try_new_array(dims, component_type.clone().to_array_field_type(i), RwLock::new(content.clone()))?;
                local_content.push(Value::Reference(arr_ref.id))
            }
            content = local_content;
        }
        //FIXME component_type.to_array_field_type(dims) is just array_field_type
        let arr_ref = ctx.try_new_array(dims, component_type.to_array_field_type(dims), RwLock::new(content))?;
        Ok(VMResultType::Successful(Value::Reference(arr_ref.id)))
    } else {
        Err(VmError::ValidationError(format!("Field type for creating an array must be FieldType::Array but is {:?}", array_field_type)))
    }
}

#[derive(Debug, PartialEq)]
enum InvokeKind{
    STATIC,
    SPECIAL,
    VIRTUAL,
    INTERFACE,
}