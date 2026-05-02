use std::{cell::RefCell, str::FromStr};

use crate::class_file::constant_pool::{ConstantPool, ConstantPoolEntry};
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::class_manager::ClassLoadingState;
use crate::vm::jni::types::JavaVM;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::{bytecode::Instruction, get_or_init, get_or_init_option, vm::{bytecode::InstructionBlock, class::{ClassAndMethod, ClassRef}, java_error::JavaError, result::VMResult, value::{ReferenceType, Value}, VmError, VM}};
use log::{debug, error, info, trace, warn};

macro_rules! wrap_error {
    ($res:expr) => {
        match $res{
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub fn execute<'a>(vm: &VM<'a>, java_vm: &JavaVM) -> VMPartialResult<Option<Value<'a>>>{
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    if vm.class_manager.expect_class_state(class_and_method.class.id, ClassLoadingState::LOADED){
        unreachable!("Class {} has to be initialized to call {} upon", class_and_method.class.name, class_and_method.format());
    }
    info!("");
    info!("METHOD_NAME: {} at {}", class_and_method.format(), vm.call_stack.get_pc().0);
    debug!("{:?}", class_and_method.method.code_blocks);
    if let Some(_) = &class_and_method.method.attributes.code{
        let mut result = execute_current_block(vm, java_vm);
        while let None = result{
            result = execute_current_block(vm, java_vm);
        }
        return result.unwrap();
    }
    Err(VmError::MethodCallError(format!("Method: {} is not executeable, because it has no code", class_and_method.format())))
}

pub fn execute_current_block<'a>(vm: &VM<'a>, java_vm: &JavaVM) -> Option<VMPartialResult<Option<Value<'a>>>>{
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    let block = class_and_method.method.get_code_block_at(vm.call_stack.get_pc());
    let current_pc = vm.call_stack.get_pc();
    trace!(">{:03} {:?}", current_pc.0, block);
    trace!("stack[{}]=", class_and_method.get_max_stack_size());
    for (index, value) in vm.call_stack.operand_stacks.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    trace!("locals[{}]=", class_and_method.get_max_locals());
    for (index, value) in vm.call_stack.locals_stack.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    if let Some(next_pc) = class_and_method.method.next_pc(current_pc){
        vm.call_stack.set_pc(next_pc);
    }
    
    match block{
        InstructionBlock::Single(instruction) => {
            match instruction{
                Instruction::ACONST_NULL => {
                    vm.call_stack.push_operand_value(vm.null());
                }
                Instruction::ICONSTM1 => x_const(vm, Value::Integer(-1)),
                Instruction::ICONST0 => x_const(vm, Value::Integer(0)),
                Instruction::ICONST1 => x_const(vm, Value::Integer(1)),
                Instruction::ICONST2 => x_const(vm, Value::Integer(2)),
                Instruction::ICONST3 => x_const(vm, Value::Integer(3)),
                Instruction::ICONST4 => x_const(vm, Value::Integer(4)),
                Instruction::ICONST5 => x_const(vm, Value::Integer(5)),
                Instruction::LCONST0 => x_const(vm, Value::Long(0)),
                Instruction::LCONST1 => x_const(vm, Value::Long(1)),
                Instruction::FCONST0 => x_const(vm, Value::Float(0.0)),
                Instruction::FCONST1 => x_const(vm, Value::Float(1.0)),
                Instruction::FCONST2 => x_const(vm, Value::Float(2.0)),
                Instruction::DCONST0 => x_const(vm, Value::Double(0.0)),
                Instruction::DCONST1 => x_const(vm, Value::Double(1.0)),
                Instruction::BIPUSH(value) => {
                    debug!("BIPUSH {:?}", value);
                    vm.call_stack.push_operand_value(Value::Integer(*value as i32))
                }
                Instruction::SIPUSH(value) => {
                    debug!("SIPUSH {:?}", value);
                    vm.call_stack.push_operand_value(Value::Integer(*value as i32))
                }

                Instruction::LDC(index) => {
                    let value = get_or_init_option!(get_constant_as_value(vm, (*index) as u16));
                    vm.call_stack.push_operand_value(value);
                }
                Instruction::LDCW(index) => {
                    let value = get_or_init_option!(get_constant_as_value(vm, *index));
                    vm.call_stack.push_operand_value(value);
                }
                Instruction::LDC2W(index) => {
                    let value = get_or_init_option!(get_constant_as_value(vm, *index));
                    vm.call_stack.push_operand_value(value);
                }

                Instruction::ILOAD(index) => wrap_error!(iload(vm, *index as usize)),
                Instruction::LLOAD(index) => wrap_error!(lload(vm, *index as usize)),
                Instruction::FLOAD(index) => wrap_error!(fload(vm, *index as usize)),
                Instruction::DLOAD(index) => wrap_error!(dload(vm, *index as usize)),
                Instruction::ALOAD(index) => wrap_error!(aload(vm, *index as usize)),

                Instruction::ILOAD0 => wrap_error!(iload(vm, 0)),
                Instruction::ILOAD1 => wrap_error!(iload(vm, 1)),
                Instruction::ILOAD2 => wrap_error!(iload(vm, 2)),
                Instruction::ILOAD3 => wrap_error!(iload(vm, 3)),

                Instruction::LLOAD0 => wrap_error!(lload(vm, 0)),
                Instruction::LLOAD1 => wrap_error!(lload(vm, 1)),
                Instruction::LLOAD2 => wrap_error!(lload(vm, 2)),
                Instruction::LLOAD3 => wrap_error!(lload(vm, 3)),

                Instruction::FLOAD0 => wrap_error!(fload(vm, 0)),
                Instruction::FLOAD1 => wrap_error!(fload(vm, 1)),
                Instruction::FLOAD2 => wrap_error!(fload(vm, 2)),
                Instruction::FLOAD3 => wrap_error!(fload(vm, 3)),

                Instruction::DLOAD0 => wrap_error!(dload(vm, 0)),
                Instruction::DLOAD1 => wrap_error!(dload(vm, 1)),
                Instruction::DLOAD2 => wrap_error!(dload(vm, 2)),
                Instruction::DLOAD3 => wrap_error!(dload(vm, 3)),

                Instruction::ALOAD0 => wrap_error!(aload(vm, 0)),
                Instruction::ALOAD1 => wrap_error!(aload(vm, 1)),
                Instruction::ALOAD2 => wrap_error!(aload(vm, 2)),
                Instruction::ALOAD3 => wrap_error!(aload(vm, 3)),

                Instruction::IALOAD | Instruction::LALOAD | Instruction::DALOAD | Instruction::AALOAD | Instruction::BALOAD | Instruction::CALOAD | Instruction::SALOAD => {
                    let index = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    let array = vm.call_stack.pop_operand_value();
                    debug!("XALOAD: {:?}[{}]", array, index);
                    if let Some(Value::Reference(array_ref)) = array{
                        vm.call_stack.push_operand_value(array_ref.get_element(index as usize));
                    }
                }

                Instruction::ISTORE(index) => wrap_error!(istore(vm, *index as usize)),
                Instruction::LSTORE(index) => wrap_error!(lstore(vm, *index as usize)),
                Instruction::FSTORE(index) => wrap_error!(fstore(vm, *index as usize)),
                Instruction::ASTORE(index) => wrap_error!(astore(vm, *index as usize)),

                Instruction::ISTORE0 => wrap_error!(istore(vm, 0)),
                Instruction::ISTORE1 => wrap_error!(istore(vm, 1)),
                Instruction::ISTORE2 => wrap_error!(istore(vm, 2)),
                Instruction::ISTORE3 => wrap_error!(istore(vm, 3)),

                Instruction::LSTORE0 => wrap_error!(lstore(vm, 0)),
                Instruction::LSTORE1 => wrap_error!(lstore(vm, 1)),
                Instruction::LSTORE2 => wrap_error!(lstore(vm, 2)),
                Instruction::LSTORE3 => wrap_error!(lstore(vm, 3)),

                Instruction::FSTORE0 => wrap_error!(fstore(vm, 0)),
                Instruction::FSTORE1 => wrap_error!(fstore(vm, 1)),
                Instruction::FSTORE2 => wrap_error!(fstore(vm, 2)),
                Instruction::FSTORE3 => wrap_error!(fstore(vm, 3)),

                Instruction::ASTORE0 => wrap_error!(astore(vm, 0)),
                Instruction::ASTORE1 => wrap_error!(astore(vm, 1)),
                Instruction::ASTORE2 => wrap_error!(astore(vm, 2)),
                Instruction::ASTORE3 => wrap_error!(astore(vm, 3)),

                Instruction::IASTORE | Instruction::LASTORE | Instruction::FASTORE | Instruction::DASTORE | Instruction::AASTORE | Instruction::BASTORE | Instruction::CASTORE | Instruction::SASTORE => {
                    //TODO validate type of value to fit instruction
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    let index = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    let popped = vm.call_stack.pop_operand_value().unwrap();
                    debug!("XASTORE: {:?}[{}] <- {:?}", popped, index, value);
                    if let Value::Reference(array_ref) = popped{
                        array_ref.set_element(index as usize, value);
                    }
                }

                Instruction::POP => {
                    debug!("POP");
                    if vm.call_stack.pop_operand_value().is_none(){
                        return Some(Err(VmError::ValidationError("Expected a value to pop but Stack was empty".to_string())));
                    }
                }
                Instruction::POP2 => {
                    debug!("POP2");
                    let popped1 = vm.call_stack.pop_operand_value();
                    if let Some(val) = popped1{
                        if val.get_computational_type() == 1{
                            if vm.call_stack.pop_operand_value().is_none(){
                                return Some(Err(VmError::ValidationError("Expected a second value to pop but Stack was empty".to_string())));
                            }
                        }
                    } else {
                        return Some(Err(VmError::ValidationError("Expected a value to pop but Stack was empty".to_string())));
                    }
                }
                Instruction::DUP => {
                    let top = vm.call_stack.pop_operand_value().unwrap();
                    vm.call_stack.push_operand_value(top.clone());
                    vm.call_stack.push_operand_value(top);
                }
                Instruction::DUPX1 => {
                    debug!("DUPX1");
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    let value2 = vm.call_stack.pop_operand_value().unwrap();
                    vm.call_stack.push_operand_value(value.clone());
                    vm.call_stack.push_operand_value(value2);
                    vm.call_stack.push_operand_value(value);
                }
                Instruction::DUPX2 => {
                    debug!("DUPX2");
                    let value1 = vm.call_stack.pop_operand_value().unwrap();
                    let value2 = vm.call_stack.pop_operand_value().unwrap();
                    if value2.get_computational_type() == 1{
                        let value3 = vm.call_stack.pop_operand_value().unwrap();
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value3);
                        vm.call_stack.push_operand_value(value2);
                        vm.call_stack.push_operand_value(value1);
                    } else {
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value2);
                        vm.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2 => {
                    debug!("DUP2");
                    let value1 = vm.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1{
                        let value2 = vm.call_stack.pop_operand_value().unwrap();
                        vm.call_stack.push_operand_value(value2.clone());
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value2);
                        vm.call_stack.push_operand_value(value1);
                    } else {
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2X1 => {
                    debug!("DUP2X1");
                    let value1 = vm.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1{
                        let value2 = vm.call_stack.pop_operand_value().unwrap();
                        let value3 = vm.call_stack.pop_operand_value().unwrap();
                        vm.call_stack.push_operand_value(value2.clone());
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value3);
                        vm.call_stack.push_operand_value(value2);
                        vm.call_stack.push_operand_value(value1);
                    } else {
                        let value2 = vm.call_stack.pop_operand_value().unwrap();
                        vm.call_stack.push_operand_value(value1.clone());
                        vm.call_stack.push_operand_value(value2);
                        vm.call_stack.push_operand_value(value1);
                    }
                }
                Instruction::DUP2X2 => {
                    debug!("DUP2X1");
                    let value1 = vm.call_stack.pop_operand_value().unwrap();
                    let value2 = vm.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 2{
                        if value2.get_computational_type() == 2{
                            vm.call_stack.push_operand_value(value1.clone());
                            vm.call_stack.push_operand_value(value2);
                            vm.call_stack.push_operand_value(value1);
                        } else {
                            let value3 = vm.call_stack.pop_operand_value().unwrap();
                            vm.call_stack.push_operand_value(value1.clone());
                            vm.call_stack.push_operand_value(value3);
                            vm.call_stack.push_operand_value(value2);
                            vm.call_stack.push_operand_value(value1);
                        }
                    } else {
                        let value3 = vm.call_stack.pop_operand_value().unwrap();
                        if value3.get_computational_type() == 2{
                            vm.call_stack.push_operand_value(value2.clone());
                            vm.call_stack.push_operand_value(value1.clone());
                            vm.call_stack.push_operand_value(value3);
                            vm.call_stack.push_operand_value(value2);
                            vm.call_stack.push_operand_value(value1);
                        } else {
                            let value4 = vm.call_stack.pop_operand_value().unwrap();
                            vm.call_stack.push_operand_value(value2.clone());
                            vm.call_stack.push_operand_value(value1.clone());
                            vm.call_stack.push_operand_value(value4);
                            vm.call_stack.push_operand_value(value3);
                            vm.call_stack.push_operand_value(value2);
                            vm.call_stack.push_operand_value(value1);
                        }
                    }
                }
                Instruction::SWAP => {
                    debug!("SWAP");
                    let value1 = vm.call_stack.pop_operand_value().unwrap();
                    let value2 = vm.call_stack.pop_operand_value().unwrap();
                    if value1.get_computational_type() == 1 && value2.get_computational_type() == 1{
                        vm.call_stack.push_operand_value(value1);
                        vm.call_stack.push_operand_value(value2);
                    } else {
                        return Some(Err(VmError::ValidationError("SWAP can only be applied to computational type 1 values".to_string())));
                    }
                }

                Instruction::IADD => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_add(val2)))),
                Instruction::LADD => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_add(val2)))),
                Instruction::FADD => wrap_error!(execute_f_arithmetic(vm, |val1, val2| Ok(val1 + val2))),
                Instruction::DADD => wrap_error!(execute_d_arithmetic(vm, |val1, val2| Ok(val1 + val2))),

                Instruction::ISUB => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_sub(val2)))),
                Instruction::LSUB => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_sub(val2)))),
                Instruction::FSUB => wrap_error!(execute_f_arithmetic(vm, |val1, val2| Ok(val1 - val2))),
                Instruction::DSUB => wrap_error!(execute_d_arithmetic(vm, |val1, val2| Ok(val1 - val2))),

                Instruction::IMUL => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::LMUL => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::FMUL => wrap_error!(execute_f_arithmetic(vm, |val1, val2| Ok(val1 * val2))),
                Instruction::DMUL => wrap_error!(execute_d_arithmetic(vm, |val1, val2| Ok(val1 * val2))),

                Instruction::IDIV => wrap_error!(execute_i_arithmetic(vm, |val1, val2| {
                    if val2 != 0 {
                        Ok(val1.wrapping_div(val2))
                    } else {
                        Err(VmError::JavaException(JavaError::DivisionByZero))
                    }
                })),
                Instruction::FDIV => wrap_error!(execute_f_arithmetic(vm, |val1, val2| {
                    if val2 != 0.0 {
                        Ok(val1 / val2)
                    } else {
                        Err(VmError::JavaException(JavaError::DivisionByZero))
                    }
                })),

                Instruction::IREM => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_rem(val2)))),
                Instruction::LREM => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_rem(val2)))),

                Instruction::INEG => {
                    let value = wrap_error!(vm.call_stack.pop_operand_value().unwrap().expect_int());
                    vm.call_stack.push_operand_value(Value::Integer(-value))
                }

                Instruction::ISHL => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 << (val2 & 0x1f)))),
                Instruction::LSHL => wrap_error!(execute_ji_arithmetic(vm, |val1, val2| Ok(val1 << (val2 & 0x3f)))),
                Instruction::ISHR => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 >> (val2 & 0x1f)))),
                Instruction::LSHR => wrap_error!(execute_ji_arithmetic(vm, |val1, val2| Ok(val1 >> (val2 & 0x3f)))),
                Instruction::IUSHR => wrap_error!(execute_i_arithmetic(vm, |val1, val2| {
                    if val1 > 0{
                        Ok(val1 >> (val2 & 0x1f))
                    } else {
                        Ok(((val1 as u32) >> (val2 & 0x1f)) as i32)
                    }
                })),
                Instruction::LUSHR => wrap_error!(execute_ji_arithmetic(vm, |val1, val2| {
                    if val1 > 0{
                        Ok(val1 >> (val2 & 0x1f))
                    } else {
                        Ok(((val1 as u64) >> (val2 & 0x1f)) as i64)
                    }
                })),

                Instruction::IAND => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 & val2))),
                Instruction::LAND => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1 & val2))),
                Instruction::IOR  => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 | val2))),
                Instruction::LOR  => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1 | val2))),
                Instruction::IXOR => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 ^ val2))),
                Instruction::LXOR => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1 ^ val2))),
                Instruction::IINC(index, amount) => {
                    if let Some(Value::Integer(value)) = vm.call_stack.load_local(*index as usize){
                        vm.call_stack.store_local(Value::Integer(value + *amount as i32), *index as usize);
                    }
                }

                //TODO fix conversions to work always
                Instruction::I2L => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2L");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Long(val as i64));
                    } else {
                        warn!("I2L Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2F => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2F");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Float(val as f32));
                    } else {
                        warn!("I2F Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2D => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2D");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Double(val as f64));
                    } else {
                        warn!("I2D Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::L2I => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("L2I");
                    if let Value::Long(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("L2I Conversion failed, because {value:?} is not of type Long")
                    }
                }
                Instruction::L2F => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("L2F");
                    if let Value::Long(val) = value {
                        vm.call_stack.push_operand_value(Value::Float(val as f32));
                    } else {
                        warn!("L2F Conversion failed, because {value:?} is not of type Long")
                    }
                }
                Instruction::F2I => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("F2I");
                    if let Value::Float(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("F2I Conversion failed, because {value:?} is not of type Float")
                    }
                }
                Instruction::F2D => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("F2D");
                    if let Value::Float(val) = value {
                        vm.call_stack.push_operand_value(Value::Double(val as f64));
                    } else {
                        warn!("F2D Conversion failed, because {value:?} is not of type Float")
                    }
                }
                Instruction::D2I => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("D2I");
                    if let Value::Double(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("D2I Conversion failed, because {value:?} is not of type Double")
                    }
                }
                Instruction::D2L => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("D2L");
                    if let Value::Double(val) = value {
                        vm.call_stack.push_operand_value(Value::Long(val as i64));
                    } else {
                        warn!("D2L Conversion failed, because {value:?} is not of type Double")
                    }
                }
                Instruction::I2B => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2B");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer((val as u8) as i32));
                    } else {
                        warn!("I2B Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2C => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2C");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer((val as u16) as i32));
                    } else {
                        warn!("I2C Conversion failed, because {value:?} is not of type Integer")
                    }
                }
                Instruction::I2S => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("I2S");
                    if let Value::Integer(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer((val as i16) as i32));
                    } else {
                        warn!("I2S Conversion failed, because {value:?} is not of type Integer")
                    }
                }

                Instruction::LCMP => {
                    if let (Some(Value::Long(value2)), Some(Value::Long(value1))) = (vm.call_stack.pop_operand_value(), vm.call_stack.pop_operand_value()) {
                        debug!("LCMP");
                        if value1 > value2 {
                            vm.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            vm.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            vm.call_stack.push_operand_value(Value::Integer(-1))
                        }
                    }
                }
                Instruction::FCMPG | Instruction::FCMPL => {
                    if let (Some(Value::Float(value2)), Some(Value::Float(value1))) = (vm.call_stack.pop_operand_value(), vm.call_stack.pop_operand_value()) {
                        debug!("FCMP");
                        if value1 > value2 {
                            vm.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            vm.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            vm.call_stack.push_operand_value(Value::Integer(-1))
                        } else if value1.is_nan() || value2.is_nan() {
                            if instruction == &Instruction::FCMPG {
                                vm.call_stack.push_operand_value(Value::Integer(1))
                            } else {
                                vm.call_stack.push_operand_value(Value::Integer(-1))
                            }
                        }
                    }
                }
                Instruction::DCMPG | Instruction::DCMPL => {
                    if let (Some(Value::Double(value2)), Some(Value::Double(value1))) = (vm.call_stack.pop_operand_value(), vm.call_stack.pop_operand_value()) {
                        debug!("DCMP");
                        if value1 > value2 {
                            vm.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2 {
                            vm.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2 {
                            vm.call_stack.push_operand_value(Value::Integer(-1))
                        } else if value1.is_nan() || value2.is_nan() {
                            if instruction == &Instruction::DCMPG {
                                vm.call_stack.push_operand_value(Value::Integer(1))
                            } else {
                                vm.call_stack.push_operand_value(Value::Integer(-1))
                            }
                        }
                    }
                }

                Instruction::IFEQ(target) => { execute_cmp(vm, *target, |value| value == 0) }
                Instruction::IFNE(target) => { execute_cmp(vm, *target, |value| value != 0) }
                Instruction::IFLT(target) => { execute_cmp(vm, *target, |value| value <  0) }
                Instruction::IFGE(target) => { execute_cmp(vm, *target, |value| value >= 0) }
                Instruction::IFGT(target) => { execute_cmp(vm, *target, |value| value >  0) }
                Instruction::IFLE(target) => { execute_cmp(vm, *target, |value| value <= 0) }

                Instruction::IF_ICMPNE(target) => execute_i_cmp(vm, *target, |val1, val2| val1 != val2),
                Instruction::IF_ICMPGT(target) => execute_i_cmp(vm, *target, |val1, val2| val1 >  val2),
                Instruction::IF_ICMPGE(target) => execute_i_cmp(vm, *target, |val1, val2| val1 >= val2),
                Instruction::IF_ICMPEQ(target) => execute_i_cmp(vm, *target, |val1, val2| val1 == val2),
                Instruction::IF_ICMPLT(target) => execute_i_cmp(vm, *target, |val1, val2| val1 <  val2),
                Instruction::IF_ICMPLE(target) => execute_i_cmp(vm, *target, |val1, val2| val1 <= val2),

                Instruction::IF_ACMPEQ(target) => {
                    let o1 = vm.call_stack.pop_operand_value().unwrap();
                    let o2 = vm.call_stack.pop_operand_value().unwrap();
                    match (o1, o2) {
                        (Value::Reference(obj1), Value::Reference(obj2)) => {
                            debug!("IF_ACMPEQ {:?} == {:?}?", obj1.id, obj2.id);
                            if obj1.id == obj2.id {
                                vm.call_stack.set_pc(*target);
                            }
                        }
                        _ => {}
                    };
                }
                Instruction::IF_ACMPNE(target) => {
                    let o1 = vm.call_stack.pop_operand_value().unwrap();
                    let o2 = vm.call_stack.pop_operand_value().unwrap();
                    match (o1, o2) {
                        (Value::Reference(obj1), Value::Reference(obj2)) => {
                            debug!("IF_ACMPNE {:?} != {:?}?", obj1.id, obj2.id);
                            if obj1.id != obj2.id {
                                vm.call_stack.set_pc(*target);
                            }
                        }
                        _ => {}
                    };
                }

                Instruction::GOTO(target) => vm.call_stack.set_pc(*target),

                Instruction::TABLESWITCH(default, low, high, offsets) => {
                    let index = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    if index < *low || index > *high{
                        debug!("TABLESWITCH default {}", default);
                        vm.call_stack.set_pc((current_pc.0 as i32 + default) as u16);
                    } else {
                        let offset = offsets[(index - low) as usize];
                        debug!("TABLESWITCH[{}]: {}", index, offset);
                        vm.call_stack.set_pc((current_pc.0 as i32 + offset) as u16);
                    }
                }
                Instruction::LOOKUPSWITCH(default, pair_stream) => {
                    let popped = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
                    debug!("LOOKUPSWITCH: {}", popped);
                    let mut use_default = true;
                    for chunk in pair_stream.chunks(2){
                        let (int_match, target) = (chunk[0], chunk[1]);
                        if int_match == popped{
                            vm.call_stack.set_pc(target as u16);
                            use_default = false;
                            break;
                        }
                    }
                    if use_default{
                        vm.call_stack.set_pc(*default as u16);
                    }
                }

                Instruction::IRETURN | Instruction::LRETURN | Instruction::FRETURN | Instruction::DRETURN | Instruction::ARETURN => {
                    //TODO seperate for validation
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    if !class_and_method.method.is_static(){
                        if let Some(Value::Reference(this)) = vm.call_stack.load_local(0){
                            vm.debug_helper.tracker.push_object_event(this.id, format!("Function {} returned:\n    {:?}", class_and_method.format(), value))
                        }
                    }
                    vm.debug_helper.tracker.push_method_event(class_and_method.format(), format!("returning: {:?}", value));
                    if !class_and_method.method.descriptor.return_type.clone().map(|rt| rt == value).unwrap_or(false) {
                        unreachable!("Trying to return {:?} but expecting: {:?}", value, class_and_method.method.descriptor.return_type)
                    }
                    return Some(Ok(VMResultType::Successful(Some(value))))
                }
                Instruction::RETURN => {
                    vm.debug_helper.tracker.push_method_event(class_and_method.format(), "returning".to_string());
                    if class_and_method.method.name == "<clinit>"{
                        vm.class_manager.update_class_state(class_and_method.class, ClassLoadingState::INITIALIZED);
                    }
                    return Some(Ok(VMResultType::Successful(None)))
                }

                Instruction::PUTSTATIC(index) => {
                    let caf = class_and_method.get_constant_field_ref(&vm, *index).unwrap();
                    let (field_index, info, class_id) = class_and_method.class.find_field_static(caf.field.name.as_str()).unwrap();
                    if vm.class_manager.expect_class_state(class_id, ClassLoadingState::LOADED){
                        unimplemented!()
                    }
                    debug!("PUTSTATIC {} {} {} {:?}", caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    //let class_id = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap().id;
                    let object = vm.get_static_class_object(class_id).unwrap();
                    vm.debug_helper.tracker.push_object_event(object.id, format!("Set static field: {}: {:?} to:\n    {:?}", info.name, info.field_type, value));
                    object.set_field(field_index, value);
                }
                Instruction::GETSTATIC(index) => {
                    let caf = class_and_method.get_constant_field_ref(&vm, *index).unwrap();
                    //let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                    //let class = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap();
                    let class = get_or_init_option!(vm.get_or_initialize_class(caf.class.name.as_str()));
                    if vm.class_manager.expect_class_state(class.id, ClassLoadingState::LOADED){
                        unimplemented!()
                    }
                    let (field_index, info, class_id) = class.find_field_static(caf.field.name.as_str()).unwrap();
                    let object = vm.get_static_class_object(class_id).unwrap();
                    debug!("GETSTATIC {} {} {} {:?}", caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    vm.call_stack.push_operand_value(object.get_field(field_index));
                }
                Instruction::GETFIELD(index) => {
                    let caf = class_and_method.get_constant_field_ref(&vm, *index).unwrap();
                    debug!("GETFIELD {}.{} {}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor());
                    let (field_index, _) = caf.class.find_field(caf.field.name.as_str()).unwrap();
                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj) = object && !object.is_null(){
                        vm.call_stack.push_operand_value(obj.get_field(field_index));
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Cannot get field: {}.{}::{} because 'this' is {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), object))));
                    }
                }
                Instruction::PUTFIELD(index) => {
                    let caf = class_and_method.get_constant_field_ref(&vm, *index).unwrap();
                    let (field_index, info) = caf.class.find_field(caf.field.name.as_str()).unwrap();
                    debug!("PUTFIELD {}.{} {} {} {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), field_index, info);
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj) = object && !object.is_null(){
                        vm.debug_helper.tracker.push_object_event(obj.id, format!("Set field: {}: {:?} to:\n    {:?}", info.name, info.field_type, value));
                        obj.set_field(field_index, value);
                        debug!("obj:{:?}", &obj);
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Cannot get field: {}.{}::{} because 'this' is {:?}", caf.class.name, caf.field.name, caf.field.field_type.to_descriptor(), object))));
                    }
                }

                Instruction::INVOKEVIRTUAL(index) => { return Some(execute_invoke(vm, *index, InvokeKind::VIRTUAL)) }
                Instruction::INVOKESPECIAL(index) => { return Some(execute_invoke(vm, *index, InvokeKind::SPECIAL)) }
                Instruction::INVOKESTATIC(index) => { return Some(execute_invoke(vm, *index, InvokeKind::STATIC)) }
                Instruction::INVOKEINTERFACE(index, _, _) => { return Some(execute_invoke(vm, *index, InvokeKind::INTERFACE)) }
                Instruction::INVOKEDYNAMIC(index, _, _) => {
                    if let Some(ConstantPoolEntry::InvokeDynamic(bm, method_name, type_name)) = class_and_method.class.get_or_resolve_constant(vm, *index){
                        if let Some(ConstantPoolEntry::MethodHandleMethod(kind, cam)) = class_and_method.class.get_or_resolve_constant(vm, bm.bootstrap_method_ref){
                            let method_type_ref_option = get_or_init_option!(vm.new_method_type(java_vm, &cam.method.descriptor));

                            if let Some(Value::Reference(method_type_ref)) = method_type_ref_option{
                                let method_handle_option = get_or_init_option!(vm.new_method_handle(java_vm, class_and_method.class, kind, cam, method_type_ref));
                                unimplemented!()
                            }
                        }
                    }
                    unimplemented!()
                }

                Instruction::NEW(index) => {
                    let class = class_and_method.get_constant_class_ref(vm, *index).unwrap();
                    let class_ref = get_or_init_option!(vm.get_or_initialize_class(class.name.as_str()));
                    if vm.class_manager.expect_class_state(class_ref.id, ClassLoadingState::LOADED){
                        unimplemented!("Cannot create instance of {:?} if not initializ-ed/-ing", class_ref.name);
                    }
                    let new_object = vm.new_object_from_class(class_ref);

                    debug!("NEW: {} {} {:?}", index, class.name, &new_object);
                    vm.call_stack.push_operand_value(Value::Reference(new_object));
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
                    let array = get_or_init_option!(execute_create_array(vm, array_field_type, 1));

                    debug!("NEWARRAY {}", atype);
                    vm.call_stack.push_operand_value(array);
                }
                Instruction::ANEWARRAY(index) => {
                    let class = class_and_method.get_constant_class_ref(vm, *index).unwrap();
                    let array_field_type = FieldType::Object(class.name.clone()).to_array_field_type(1);
                    let array = get_or_init_option!(execute_create_array(vm, array_field_type, 1));
                    
                    debug!("ANEWARRAY {}", class.name);
                    vm.call_stack.push_operand_value(array);
                }
                Instruction::ARRAYLENGTH => {
                    debug!("ARRAYLENGTH");
                    let popped = vm.call_stack.pop_operand_value();
                    if let Some(Value::Reference(reference)) = popped{
                        if let ReferenceType::Array(_, _, content) = &reference.reference_type{
                            vm.call_stack.push_operand_value(Value::Integer(content.borrow().len() as i32));
                        } else {
                            return Some(Err(VmError::ValidationError(format!("Expected an Array ref but found: {:?}", reference))))
                        }
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Expected an array ref but found: {:?}", &popped))))
                    }
                }

                Instruction::ATHROW => {
                    debug!("ATHROW");
                    if let Some(Value::Reference(error)) = vm.call_stack.pop_operand_value(){
                        let string_value = error.get_field(2);
                        let string = if !string_value.is_null() {VM::extract_string_from_object(&string_value).unwrap()} else {String::new()};
                        let exception_name = vm.class_manager.find_class_by_id(error.class_id).unwrap().name.clone();
                        vm.debug_helper.exception_helper.push(format!("Throw   {}: {}\n└-- thrown by {} at {}", exception_name, string, class_and_method.format(), vm.call_stack.get_pc().0));
                        let prev = vm.caught_exception.replace(Some((string, class_and_method.format(), Value::Reference(error))));
                        assert!(prev.is_none());
                        return Some(Ok(VMResultType::ExceptionThrown));
                    }
                    return Some(Err(VmError::JavaException(JavaError::JavaExceptionThrown("JavaException".to_string(), "Unknown".to_string(), class_and_method.format()))));
                }

                Instruction::CHECKCAST(constant_index) => {
                    //TODO
                    debug!("CHECKCAST {:?}", &class_and_method.class.get_or_resolve_constant(&vm, *constant_index));
                }
                Instruction::INSTANCEOF(constant_index) => {
                    let of_class = match class_and_method.class.get_or_resolve_constant(vm, *constant_index){
                        Some(ConstantPoolEntry::Class(class_ref)) => class_ref,
                        _ => return Some(Err(VmError::ValidationError("Expected a resolvable class entry".to_string()))),
                    };

                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if object.is_null(){
                        vm.call_stack.push_operand_value(Value::from(false));
                        return None;
                    }
                    let object = object.expect_reference().unwrap();
                    let object_class = vm.find_class_by_id(object.class_id).unwrap();
                    let instance_of = vm.is_instance_of(object_class, of_class);

                    debug!("INSTANCEOF {:?} = {}", &class_and_method.class.get_or_resolve_constant(&vm, *constant_index), instance_of);

                    vm.call_stack.push_operand_value(Value::from(instance_of));
                }

                Instruction::MONITORENTER => {
                    if let Some(Value::Reference(_)) = vm.call_stack.pop_operand_value(){
                        debug!("MONITORENTER")
                    } else {
                        warn!("No object to lock")
                    }
                }
                Instruction::MONITOREXIT => {
                    if let Some(Value::Reference(_)) = vm.call_stack.pop_operand_value(){
                        debug!("MONITOREXIT")
                    } else {
                        warn!("No object to lock")
                    }
                }

                Instruction::WIDE(op, index, const_option) => {
                    match Instruction::from_repr(*op).unwrap(){
                        unknown => unreachable!("WIDE with op: {:?} not executable", unknown)
                    }
                }
                Instruction::MULTIANEWARRAY(index, dimensions ) => {
                    if let Some(ConstantPoolEntry::Class(clazz)) = class_and_method.class.get_or_resolve_constant(vm, *index){
                        let class_name = clazz.name.as_str();
                        let array_field_type = FieldType::from_str(class_name).unwrap();
                        let array = get_or_init_option!(execute_create_array(vm, array_field_type, *dimensions as usize));
                        debug!("MULTIANEWARRAY {}", class_name);
                        vm.call_stack.push_operand_value(array);
                    }
                }

                Instruction::IFNULL(target) => {
                    let reference = vm.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Reference(r) => {
                            if r.is_null(){
                                debug!("+IFNULL is NULL");
                                vm.call_stack.set_pc(*target);
                            } else {
                                debug!("-IFNULL is reference");
                            }
                        }
                        _ => {warn!("?IFNULL {:?} is this valid?", reference.clone())}
                    }
                }
                Instruction::IFNONNULL(target) => {
                    let reference = vm.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Reference(r) => {
                            if r.is_null(){
                                debug!("-IFNONNULL is NULL");
                            } else {
                                debug!("+IFNONNULL is reference");
                                vm.call_stack.set_pc(*target);
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
            let top = vm.call_stack.operand_stacks.borrow().last().unwrap().last().unwrap().clone();
            vm.call_stack.store_local(top, *index);
        }
        InstructionBlock::IConstReturn(val) => {
            vm.debug_helper.tracker.push_method_event(class_and_method.format(), format!("returning int: {}", val));
            return Some(Ok(VMResultType::Successful(Some(Value::Integer(*val)))))
        }
        other => {
            return Some(Err(VmError::Unspecified(format!("Block of type {:?} not executable", other))))
        }
    }
    debug!("");
    None
}

fn x_const<'a>(vm: &VM<'a>, value: Value<'a>){
    vm.call_stack.push_operand_value(value);
}

fn istore(vm: &VM, index: usize) -> VMResult<()> {
    let value = vm.call_stack.pop_operand_value().unwrap();
    debug!("ISTORE{} {:?}", index, value);
    vm.call_stack.store_local(value, index);
    Ok(())
}

//TODO validation
fn lstore(vm: &VM, index: usize) -> VMResult<()> {
    let value = vm.call_stack.pop_operand_value().unwrap();
    debug!("LSTORE{} {:?}", index, value);
    vm.call_stack.store_local(value, index);
    vm.call_stack.store_local(Value::Dummy, index+1);
    Ok(())
}

fn fstore(vm: &VM, index: usize) -> VMResult<()> {
    let value = vm.call_stack.pop_operand_value().unwrap();
    debug!("FSTORE{} {:?}", index, value);
    vm.call_stack.store_local(value, index);
    Ok(())
}

fn astore(vm: &VM, index: usize) -> VMResult<()> {
    let value = vm.call_stack.pop_operand_value().unwrap();
    debug!("ASTORE{} {:?}", index, value);
    vm.call_stack.store_local(value, index);
    Ok(())
}

fn iload(vm: &VM, index: usize) -> VMResult<()> {
    let popped = vm.call_stack.load_local(index).unwrap();
    match popped {
        Value::Integer(i) => {
            debug!("ILOAD{} {}", index, i);
        }
        _ => return Err(VmError::ValidationError(format!("ILOAD{} failed", index)))
    }
    vm.call_stack.push_operand_value(popped);
    Ok(())
}

fn lload(vm: &VM, index: usize) -> VMResult<()> {
    let local = vm.call_stack.load_local(index);
    let dummy = vm.call_stack.load_local(index + 1);
    if dummy.as_ref().unwrap() != &Value::Dummy{
        return Err(VmError::ValidationError(format!("Expected a Dummy value at {} but got {:?}",index+1, dummy.unwrap())));
    }
    if let Some(Value::Long(value)) = local{
        vm.call_stack.push_operand_value(Value::Long(value));
        debug!("LLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("LLOAD{} failed, because locals[{}] was {:?} and not Long", index, index, local)))
    }
}

fn fload(vm: &VM, index: usize) -> VMResult<()> {
    let local = vm.call_stack.load_local(index);
    if let Some(Value::Float(value)) = local{
        vm.call_stack.push_operand_value(Value::Float(value));
        debug!("FLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("FLOAD{} failed, because locals[{}] was {:?} and not Float", index, index, local)))
    }
}

fn dload(vm: &VM, index: usize) -> VMResult<()> {
    let local = vm.call_stack.load_local(index);
    if let Some(Value::Double(value)) = local{
        vm.call_stack.push_operand_value(Value::Double(value));
        debug!("DLOAD{} {:?}", index, value);
        Ok(())
    } else {
        Err(VmError::ValidationError(format!("DLOAD{} failed, because locals[{}] was {:?} and not Double", index, index, local)))
    }
}

fn aload<'a>(vm: &VM<'a>, index: usize) -> VMResult<()>{
    let popped = vm.call_stack.load_local(index).unwrap();
    match popped {
        Value::Reference(reference) => {
            debug!("ALOAD{} {:?}", index, reference);
        }
        _ => return Err(VmError::ValidationError(format!("ALOAD{} failed", index)))
    }
    vm.call_stack.push_operand_value(popped);
    Ok(())
}

fn execute_cmp<F: FnOnce(i32) -> bool>(vm: &VM, target: u16, cmp: F){
    let value = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    if cmp(value){
        vm.call_stack.set_pc(target);
    }
}

fn execute_i_cmp<F: FnOnce(i32, i32) -> bool>(vm: &VM, offset: u16, f: F){
    let val2 = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    let val1 = vm.call_stack.pop_operand_value().unwrap().expect_int().unwrap();
    let jump = f(val1, val2);
    debug!("ICMP: {}&{}={}", val1, val2, jump);
    if jump{
        vm.call_stack.set_pc(offset);
    }
}

fn execute_i_arithmetic<F: FnOnce(i32, i32) -> VMResult<i32>>(vm: &VM, f: F) -> VMResult<()> {
    let value2 = vm.call_stack.pop_operand_value();
    let value1 = vm.call_stack.pop_operand_value();
    if let (Some(Value::Integer(val1)), Some(Value::Integer(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Integer ARITHMETIC {}&{}={}", val1, val2, res);
        vm.call_stack.push_operand_value(Value::Integer(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee ints to keck");
        Err(VmError::ValidationError("Expected two ints".to_string()))
    }
}

fn execute_l_arithmetic<F: FnOnce(i64, i64) -> VMResult<i64>>(vm: &VM, f: F) -> VMResult<()> {
    let value2 = vm.call_stack.pop_operand_value();
    let value1 = vm.call_stack.pop_operand_value();
    if let (Some(Value::Long(val1)), Some(Value::Long(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Long ARITHMETIC {}&{}={}", val1, val2, res);
        vm.call_stack.push_operand_value(Value::Long(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee longs to keck");
        Err(VmError::ValidationError("Expected two longs".to_string()))
    }
}

fn execute_f_arithmetic<F: FnOnce(f32, f32) -> VMResult<f32>>(vm: &VM, f: F) -> VMResult<()> {
    let value2 = vm.call_stack.pop_operand_value();
    let value1 = vm.call_stack.pop_operand_value();
    if let (Some(Value::Float(val1)), Some(Value::Float(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Float ARITHMETIC {}&{}={}", val1, val2, res);
        vm.call_stack.push_operand_value(Value::Float(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee floats to keck");
        Err(VmError::ValidationError("Expected two floats".to_string()))
    }
}

fn execute_d_arithmetic<F: FnOnce(f64, f64) -> VMResult<f64>>(vm: &VM, f: F) -> VMResult<()> {
    let value2 = vm.call_stack.pop_operand_value();
    let value1 = vm.call_stack.pop_operand_value();
    if let (Some(Value::Double(val1)), Some(Value::Double(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("Double ARITHMETIC {}&{}={}", val1, val2, res);
        vm.call_stack.push_operand_value(Value::Double(res));
        Ok(())
    } else {
        warn!("dat sin nich zwee doubles to keck");
        Err(VmError::ValidationError("Expected two doubles".to_string()))
    }
}

fn execute_ji_arithmetic<F: FnOnce(i64, i32) -> Result<i64, VmError>>(vm: &VM, f: F) -> VMResult<()> {
    let value2 = vm.call_stack.pop_operand_value();
    let value1 = vm.call_stack.pop_operand_value();
    if let (Some(Value::Long(val1)), Some(Value::Integer(val2))) = (value1, value2){
        let res = f(val1, val2)?;
        debug!("LongInt ARITHMETIC {}&{}={}", val1, val2, res);
        vm.call_stack.push_operand_value(Value::Long(res));
        Ok(())
    } else {
        warn!("dat sin nich eene long und eene int du keck");
        Err(VmError::ValidationError("Expected an int and a long".to_string()))
    }
}

fn execute_invoke<'a>(vm: &VM<'a>, index: u16, kind: InvokeKind) -> VMPartialResult<Option<Value<'a>>> {
    let calling_class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    let cam = calling_class_and_method.get_constant_method_ref_fast(vm, index).expect("GIB MICH DIE METHODE");
    trace!("loading class to execute on: '{}'", cam.class.name.as_str());
    let class = get_or_init!(vm.get_or_initialize_class(cam.class.name.as_str())?);
    if vm.class_manager.expect_class_state(class.id, ClassLoadingState::LOADED){
        unimplemented!()
    }
    trace!("loading state is: {:?}", vm.class_manager.class_loading_states.borrow().get(&class.id));
    trace!("finished loading class to execute on: '{}'", cam.class.name.as_str());
    let args_count = cam.method.descriptor.args.len();
    trace!("args_count: {}", args_count);
    let mut args = Vec::new();
    for _ in 0..args_count{
        let popped = vm.call_stack.pop_operand_value().unwrap();
        match popped {
            Value::Long(_) | Value::Double(_) => {args.insert(0, Value::Dummy)}
            _ => {}
        }
        args.insert(0, popped);
    }

    let class_and_method = match kind {
        InvokeKind::SPECIAL | InvokeKind::STATIC => {
            class
                .find_method(cam.method.name.as_str(), cam.method.descriptor.as_str())
                .map(|method| ClassAndMethod {class, method})
                .unwrap_or(get_method_virtual(class, cam.method.name.as_str(), cam.method.descriptor.as_str())?)
        }
        InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
            get_method_virtual(class, cam.method.name.as_str(), cam.method.descriptor.as_str())?
        }
    };
    let receiver = if class_and_method.method.is_static(){
        None
    } else {
        let popped = vm.call_stack.pop_operand_value();
        if let Some(Value::Reference(reference)) = popped && !reference.is_null(){
            Some(reference)
        } else {
            println!("XXXX: {} {:?}", class_and_method.class.name, vm.class_manager.class_loading_states.borrow().get(&class_and_method.class.id));
            return Err(VmError::ValidationError(format!("Expected object or array as receiver for {} but found: {:?}", class_and_method.format(), popped)));
        }
    };
    let class_and_method = match kind {
        InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
            match receiver {
                Some(obj) => {
                    let receiver_class = vm.find_class_by_id(obj.class_id).unwrap();
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
    for (index, value) in vm.call_stack.operand_stacks.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    trace!("locals=");
    for (index, value) in vm.call_stack.locals_stack.borrow().last().unwrap().iter().enumerate(){
        trace!("    [{}] {:?}", index, value);
    }
    debug!("INVOKE{:?}: {}{} on {:?}", kind, cam.method.name, cam.method.descriptor.as_str(), receiver);
    if let Some(rec) = receiver{
        vm.debug_helper.tracker.push_object_event(rec.id, format!("Preparing call {} with args:{}", class_and_method.format(), args.iter().map(|v| format!("\n    {:?}", v)).collect::<Vec<_>>().join("")));
        vm.debug_helper.tracker.push_method_event(class_and_method.format(), format!("Calling on {:?} from {} with args: {}", rec, calling_class_and_method.format(), args.iter().map(|v| format!("\n    {:?}", v)).collect::<Vec<_>>().join("") ));
    } else {
        vm.debug_helper.tracker.push_method_event(class_and_method.format(), format!("Calling static from {} with args: {}", calling_class_and_method.format(), args.iter().map(|v| format!("\n    {:?}", v)).collect::<Vec<_>>().join("") ));
    }
    for (i, provided_arg) in args.iter().filter(|a| if let Value::Dummy = a {false} else {true}).enumerate(){
        if !(&class_and_method.method.descriptor.args[i] == provided_arg){
            return Err(VmError::ValidationError(format!("Expected arg type: {:?} but got value: {:?}", class_and_method.method.descriptor.args[i], provided_arg)));
        }
    }
    vm.call_stack.create_and_push_call_frame(class_and_method, receiver, args, true);
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

//FIXME: Deprecated
fn get_constant_as_value<'a>(vm: &VM<'a>, index: u16) -> VMPartialResult<Value<'a>>{
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    let constant_value = class_and_method.class.get_or_resolve_constant(&vm, index).unwrap();
    let value = match constant_value {
        ConstantPoolEntry::Integer(value) => Value::Integer(value),
        ConstantPoolEntry::Long(value) => Value::Long(value),
        ConstantPoolEntry::Float(value) => Value::Float(value),
        ConstantPoolEntry::Double(value) => Value::Double(value),
        ConstantPoolEntry::String(string) => {
            let string_object = get_or_init!(vm.new_string_object(string.as_str())?);
            Value::Reference(string_object)
        }
        ConstantPoolEntry::Class(clazz) => {
            let class_object = get_or_init!(vm.new_class_object_by_class(clazz)?);
            Value::Reference(class_object)
        }
        _ => unimplemented!("Constant of type {constant_value:?} cannot be converted to a value")
    };
    Ok(VMResultType::Successful(value))
}

fn execute_create_array<'a>(vm: &VM<'a>, array_field_type: FieldType, dims: usize) -> VMPartialResult<Value<'a>>{
    if let FieldType::Array(_, component_type) = array_field_type{
        //ensure that the array class get loaded before popping the count(s)
        for i in 0..dims{
            let _ = vm.get_or_resolve_class(component_type.clone().to_array_field_type(i+1).to_class_name().as_str())?;
        }
        let mut content = Vec::new();
        for i in 0..dims{
            let current_dim = vm.call_stack.pop_operand_value().unwrap().expect_int()?;
            if current_dim == 0{
                break;
            }
            let mut local_content = Vec::new();
            if i == 0{
                local_content = vec![component_type.get_default_value(vm.null()); current_dim as usize];
                content = local_content;
                continue
            }
            for _ in 0..current_dim{
                local_content.push(Value::Reference(vm.try_new_array(dims, component_type.clone().to_array_field_type(i), RefCell::new(content.clone()))?))
            }
            content = local_content;
        }
        //FIXME component_type.to_array_field_type(dims) is just array_field_type
        Ok(VMResultType::Successful(Value::Reference(vm.try_new_array(dims, component_type.to_array_field_type(dims), RefCell::new(content))?)))
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