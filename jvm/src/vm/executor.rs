use std::{cell::RefCell, str::FromStr};

use crate::class_file::get_constant_printable;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::{bytecode::Instruction, constants::ConstantPoolEntry, field_info::{FieldType, PrimitiveType}, get_or_init, get_or_init_option, method_info::MethodDescriptor, vm::{bytecode::InstructionBlock, class::{ClassAndMethod, ClassRef}, java_error::JavaError, result::VMResult, value::{ReferenceType, Value}, VmError, VM}};
use log::{debug, error, info, trace, warn};

macro_rules! wrap_error {
    ($res:expr) => {
        match $res{
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub fn execute<'a>(vm: &VM<'a>) -> VMPartialResult<'a, Option<Value<'a>>>{
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    info!("");
    info!("METHOD_NAME: {} at {}", class_and_method.format(), vm.call_stack.get_pc().0);
    debug!("{:?}", class_and_method.method.code_blocks);
    if let Some(_) = &class_and_method.method.code{
        let mut result = execute_current_block(vm);
        while let None = result{
            result = execute_current_block(vm);
        }
        return result.unwrap();
    }
    Err(VmError::MethodCallError(format!("Method: {} is not executeable, because it has no code", class_and_method.format())))
}

pub fn execute_current_block<'a>(vm: &VM<'a>) -> Option<VMPartialResult<'a, Option<Value<'a>>>>{
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
                    vm.call_stack.push_operand_value(Value::Null);
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

                Instruction::IALOAD | Instruction::AALOAD | Instruction::BALOAD | Instruction::CALOAD => {
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

                Instruction::IASTORE | Instruction::AASTORE | Instruction::CASTORE | Instruction::BASTORE | Instruction::SASTORE => {
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

                Instruction::IADD => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_add(val2)))),
                Instruction::LADD => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_add(val2)))),

                Instruction::ISUB => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_sub(val2)))),
                Instruction::LSUB => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_sub(val2)))),

                Instruction::IMUL => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::LMUL => wrap_error!(execute_l_arithmetic(vm, |val1, val2| Ok(val1.wrapping_mul(val2)))),
                Instruction::FMUL => wrap_error!(execute_f_arithmetic(vm, |val1, val2| Ok(val1 * val2))),

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

                Instruction::ISHL => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 << (val2 & 0x1f)))),
                Instruction::LSHL => wrap_error!(execute_ji_arithmetic(vm, |val1, val2| Ok(val1 << (val2 & 0x3f)))),
                Instruction::ISHR => wrap_error!(execute_i_arithmetic(vm, |val1, val2| Ok(val1 >> (val2 & 0x1f)))),
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
                Instruction::L2I => {
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    debug!("L2I");
                    if let Value::Long(val) = value {
                        vm.call_stack.push_operand_value(Value::Integer(val as i32));
                    } else {
                        warn!("L2I Conversion failed, because {value:?} is not of type Long")
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
                    if let (Some(Value::Float(value2)), Some(Value::Float(value1))) = (vm.call_stack.pop_operand_value(), vm.call_stack.pop_operand_value()){
                        debug!("FCMP");
                        if value1 > value2{
                            vm.call_stack.push_operand_value(Value::Integer(1))
                        } else if value1 == value2{
                            vm.call_stack.push_operand_value(Value::Integer(0))
                        } else if value1 < value2{
                            vm.call_stack.push_operand_value(Value::Integer(-1))
                        } else if value1.is_nan() || value2.is_nan(){
                            if instruction == &Instruction::FCMPG{
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
                        (Value::Null, Value::Null) => vm.call_stack.set_pc(*target),
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
                        (Value::Reference(_), Value::Null) | (Value::Null, Value::Reference(_)) => vm.call_stack.set_pc(*target),
                        _ => {}
                    };
                }

                Instruction::GOTO(target) => vm.call_stack.set_pc(*target),

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

                Instruction::IRETURN | Instruction::LRETURN | Instruction::FRETURN | Instruction::ARETURN => {
                    //TODO seperate for validation
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    return Some(Ok(VMResultType::Ok(Some(value))))
                }
                Instruction::RETURN => {
                    return Some(Ok(VMResultType::Ok(None)))
                }

                Instruction::PUTSTATIC(index) => {
                    let (_class_name, field_name, descriptor) = class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD");
                    let (field_index, info, class_id) = class_and_method.class.find_field_static(field_name.as_str()).unwrap();
                    debug!("PUTSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    //let class_id = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap().id;
                    let object = vm.get_static_class_object(class_id).unwrap();
                    object.set_field(field_index, value);
                }
                Instruction::GETSTATIC(index) => {
                    let (class_name, field_name, descriptor) = class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD2");
                    //let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                    //let class = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap();
                    let class = get_or_init_option!(vm.get_or_resolve_class(class_name.as_str()));
                    let (field_index, info, class_id) = class.find_field_static(field_name.as_str()).unwrap();
                    let object = vm.get_static_class_object(class_id).unwrap();
                    debug!("GETSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                    vm.call_stack.push_operand_value(object.get_field(field_index));
                }
                Instruction::GETFIELD(index) => {
                    let (class_name, field_name, descriptor) = class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD3");
                    debug!("GETFIELD {}.{} {}", class_name, field_name, descriptor);
                    let target_class = if class_name == class_and_method.class.name {
                        class_and_method.class
                    } else {
                        get_or_init_option!(vm.get_or_resolve_class(class_name.as_str()))
                    };
                    let (field_index, _) = target_class.find_field(field_name.as_str()).unwrap();
                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj) = object {
                        vm.call_stack.push_operand_value(obj.get_field(field_index));
                    } else {
                        warn!("NAO");
                    }
                }
                Instruction::PUTFIELD(index) => {
                    let (class_name, field_name, descriptor) = class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD");
                    let target_class = if class_name == class_and_method.class.name {
                        class_and_method.class
                    } else {
                        get_or_init_option!(vm.get_or_resolve_class(class_name.as_str()))
                    };
                    let (field_index, info) = target_class.find_field(field_name.as_str()).unwrap();
                    debug!("PUTFIELD {}.{} {} {} {:?}", class_name, field_name, descriptor, field_index, info);
                    let value = vm.call_stack.pop_operand_value().unwrap();
                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if let Value::Reference(obj) = object {
                        obj.set_field(field_index, value);
                        debug!("obj:{:?}", &obj);
                    } else {
                        warn!("NAO");
                    }
                }

                Instruction::INVOKEVIRTUAL(index) => { return Some(execute_invoke(vm, *index, InvokeKind::VIRTUAL)) }
                Instruction::INVOKESPECIAL(index) => { return Some(execute_invoke(vm, *index, InvokeKind::SPECIAL)) }
                Instruction::INVOKESTATIC(index) => { return Some(execute_invoke(vm, *index, InvokeKind::STATIC)) }
                Instruction::INVOKEINTERFACE(index, _, _) => { return Some(execute_invoke(vm, *index, InvokeKind::INTERFACE)) }

                Instruction::NEW(index) => {
                    let class_name = class_and_method.get_constant_utf8(*index).unwrap();
                    let new_object = get_or_init_option!(vm.new_object(class_name.as_str()));

                    debug!("NEW: {} {} {:?}", index, class_name, &new_object);
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
                    let class_name = class_and_method.get_constant_utf8(*index).unwrap();
                    let array_field_type = FieldType::Object(class_name.clone()).to_array_field_type(1);
                    let array = get_or_init_option!(execute_create_array(vm, array_field_type, 1));
                    
                    debug!("ANEWARRAY {}", class_name);
                    vm.call_stack.push_operand_value(array);
                }
                Instruction::ARRAYLENGTH => {
                    debug!("ARRAYLENGTH");
                    let popped = vm.call_stack.pop_operand_value();
                    if let Some(Value::Reference(reference)) = popped{
                        if let ReferenceType::Array(_, _, content) = &reference.reference_type{
                            vm.call_stack.push_operand_value(Value::Integer(content.borrow().len() as i32));
                        } else {
                            return Some(Err(VmError::ValidationError("Expected an Array ref but found: Object ref".to_string())))
                        }
                    } else if let Some(Value::Null) = popped{
                        return Some(Err(VmError::JavaException(JavaError::NullPointerException("Expected an array".to_string()))))
                    } else {
                        return Some(Err(VmError::ValidationError(format!("Expected an array ref but found: {:?}", &popped))))
                    }
                }

                Instruction::ATHROW => {
                    debug!("ATHROW");
                    if let Some(Value::Reference(error)) = vm.call_stack.pop_operand_value(){
                        let string_value = error.get_field(2);
                        let string = VM::extract_string_from_object(&string_value).unwrap();
                        let exception_name = vm.class_manager.find_class_by_id(error.class_id).unwrap().name.clone();
                        #[cfg(feature = "debug")]
                        {
                            vm.debug_helper.exception_helper.push(format!("Throw   {}: {}\n└-- thrown by {} at {}", exception_name, string, class_and_method.format(), vm.call_stack.get_pc().0));
                        }
                        return Some(Ok(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(exception_name, string, class_and_method.format())), Value::Reference(error))));
                    }
                    return Some(Err(VmError::JavaException(JavaError::JavaExceptionThrown("JavaException".to_string(), "Unknown".to_string(), class_and_method.format()))));
                }

                Instruction::CHECKCAST(constant_index) => {
                    //TODO
                    debug!("CHECKCAST {}", get_constant_printable(&class_and_method.class.constants, *constant_index));
                }
                Instruction::INSTANCEOF(constant_index) => {
                    let of_class = get_or_init_option!(vm.get_or_resolve_class(get_constant_printable(&class_and_method.class.constants, *constant_index).as_str()));

                    let object = vm.call_stack.pop_operand_value().unwrap();
                    if object == Value::Null{
                        vm.call_stack.push_operand_value(Value::from(false));
                        return None;
                    }
                    let object = object.expect_reference().unwrap();
                    let object_class = vm.find_class_by_id(object.class_id).unwrap();
                    let mut instance_of = false;
                    let mut to_check = vec![object_class];
                    while let Some(next_class) = to_check.pop() {
                        if next_class.id == of_class.id{
                            instance_of = true;
                            break;
                        }
                        if let Some(super_class) = next_class.superclass{
                            to_check.push(super_class);
                        }
                        next_class.interfaces.iter().for_each(|class| to_check.push(class));
                    }

                    debug!("INSTANCEOF {} = {}", get_constant_printable(&class_and_method.class.constants, *constant_index), instance_of);

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

                Instruction::IFNULL(target) => {
                    let reference = vm.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Null => {debug!("+IFNULL is NULL"); vm.call_stack.set_pc(*target);}
                        Value::Reference(_) => {debug!("-IFNULL is reference");}
                        _ => {warn!("?IFNULL {:?} is this valid?", reference.clone())}
                    }
                }
                Instruction::IFNONNULL(target) => {
                    let reference = vm.call_stack.pop_operand_value().unwrap();
                    match reference {
                        Value::Null => {debug!("-IFNONNULL is NULL");}
                        Value::Reference(_) => {debug!("+IFNONNULL is reference"); vm.call_stack.set_pc(*target);}
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
            return Some(Ok(VMResultType::Ok(Some(Value::Integer(*val)))))
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
        Value::Null => {
            debug!("ALOAD{} (loaded null)", index);
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

fn execute_invoke<'a>(vm: &VM<'a>, index: u16, kind: InvokeKind) -> VMPartialResult<'a, Option<Value<'a>>> {
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    let (class_name, method_name, descriptor) = class_and_method.get_constant_method_info_descriptor(index).expect("GIB MICH DIE METHODE");
    trace!("loading class to execute on: '{}'", class_name.as_str());
    let class = get_or_init!(vm.get_or_resolve_class(class_name.as_str())?);
    trace!("finished loading class to execute on: '{}'", class_name.as_str());
    let args_count = MethodDescriptor::new(descriptor.clone()).args.len();
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
                .find_method(method_name.as_str(), descriptor.as_str())
                .map(|method| ClassAndMethod {class, method})
                .unwrap_or(get_method_virtual(class, method_name.as_str(), descriptor.as_str())?)
        }
        InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
            get_method_virtual(class, method_name.as_str(), descriptor.as_str())?
        }
    };
    let receiver = if class_and_method.method.is_static(){
        None
    } else {
        let popped = vm.call_stack.pop_operand_value();
        if let Some(Value::Reference(reference)) = popped{
            Some(reference)
        } else {
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
    debug!("INVOKE{:?}: {}{} on {:?}", kind, method_name, descriptor, receiver);
    let call_frame = vm.call_stack.create_and_push_call_frame(class_and_method, receiver, args, true);
    Ok(VMResultType::CallPaused(call_frame))
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

fn get_constant_as_value<'a>(vm: &VM<'a>, index: u16) -> VMPartialResult<'a, Value<'a>>{
    let class_and_method = &vm.call_stack.frames.borrow().last().unwrap().class_and_method.clone();
    let constant_value = class_and_method.class.get_constant(index).unwrap();
    let value = match constant_value {
        ConstantPoolEntry::Integer(value) => Value::Integer(value),
        ConstantPoolEntry::Long(value) => Value::Long(value),
        ConstantPoolEntry::Float(value) => Value::Float(value),
        ConstantPoolEntry::Double(value) => Value::Double(value),
        ConstantPoolEntry::String(string_index) => {
            if let Some(ConstantPoolEntry::Utf8(string)) = class_and_method.class.get_constant(string_index){
                //FIXME maybe needs preloading
                let string_object = get_or_init!(vm.new_string_object(string)?);
                Value::Reference(string_object)
            } else {
                //return Err(ValidationError("Expected string".to_string()));
                warn!("expected but didnt find string object");
                Value::Null
            }
        }
        ConstantPoolEntry::Class(name_index) => {
            if let Some(ConstantPoolEntry::Utf8(string)) = class_and_method.class.get_constant(name_index){
                let class_object = get_or_init!(vm.new_class_object_by_name(string)?);
                Value::Reference(class_object)
            } else {
                warn!("expected but didnt find string object");
                Value::Null
            }
        }
        ConstantPoolEntry::InvokeDynamic(bootstrap_method_index, name_and_type_index) => {
            if let Some(ConstantPoolEntry::NameAndType(name_index, type_index)) = class_and_method.class.get_constant(name_and_type_index){
                println!("{:?} {:?}", class_and_method.class.get_constant(name_index), class_and_method.class.get_constant(type_index))
            }
            println!("{:?}", class_and_method.class.bootstrap_methods.0.get(bootstrap_method_index as usize));
            Value::Null
        }
        _ => unimplemented!("Constant of type {constant_value:?} cannot be converted to a value")
    };
    Ok(VMResultType::Ok(value))
}

fn execute_create_array<'a>(vm: &VM<'a>, array_field_type: FieldType, dims: usize) -> VMPartialResult<'a, Value<'a>>{
    if let FieldType::Array(_, component_type) = array_field_type{
        //ensure that the array class get loaded before popping the count(s)
        for i in 0..dims{
            let _ = get_or_init!(vm.get_or_resolve_class(component_type.clone().to_array_field_type(i+1).to_class_name().as_str())?);
        }
        let mut content = Vec::new();
        for i in 0..dims{
            let current_dim = vm.call_stack.pop_operand_value().unwrap().expect_int()?;
            if current_dim == 0{
                break;
            }
            let mut local_content = Vec::new();
            if i == 0{
                local_content = vec![component_type.get_default_value(); current_dim as usize];
                content = local_content;
                continue
            }
            for _ in 0..current_dim{
                local_content.push(Value::Reference(vm.try_new_array(dims, component_type.clone().to_array_field_type(i), RefCell::new(content.clone()))?))
            }
            content = local_content;
        }
        //FIXME component_type.to_array_field_type(dims) is just array_field_type
        Ok(VMResultType::Ok(Value::Reference(vm.try_new_array(dims, component_type.to_array_field_type(dims), RefCell::new(content))?)))
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