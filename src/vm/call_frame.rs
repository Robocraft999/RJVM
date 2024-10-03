use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use log::{debug, error, info, trace, warn};
use crate::attribute::ProgramCounter;
use crate::bytecode::{Instruction, parse_instruction, printable_instructions};
use crate::class_file::get_constant_printable;
use crate::constants::ConstantPoolEntry;
use crate::field_info::{FieldType, PrimitiveType};
use crate::method_info::MethodDescriptor;
use crate::vm::java_error::JavaError;
use crate::vm::{VM, VmError};
use crate::vm::class::{ClassAndMethod, ClassRef};
use crate::vm::value::{ReferenceType, Value};

#[derive(Clone)]
pub struct CallFrame<'a>{
    pub class_and_method: ClassAndMethod<'a>,
    pub locals: Vec<Value<'a>>,
    pub pc: ProgramCounter,
    pub stack: Vec<Value<'a>>,
}

impl<'a> CallFrame<'a>{
    pub fn execute(&mut self, vm: &mut VM<'a>) -> Result<Option<Value<'a>>, VmError>{
        if let Some(code) = &self.class_and_method.method.code{
            let constants = &self.class_and_method.class.constants;
            for class in vm.class_manager.classes.iter_mut(){
                //println!("{class:?}");
            }
            info!("");
            info!("METHOD_NAME: {}.{}{}", self.class_and_method.class.name, self.class_and_method.method.name, self.class_and_method.method.descriptor.as_str());
            info!("{:?}", printable_instructions(&code.code));

            loop {
                if let Ok((instruction, pc)) = parse_instruction(&code.code, self.pc.0 as usize) {
                    self.pc = ProgramCounter(pc as u16);
                    trace!("{:?}", instruction);
                    trace!("stack=");
                    for (index, value) in self.stack.iter().enumerate(){
                        trace!("    [{}] {:?}", index, value);
                    }
                    trace!("locals=");
                    for (index, value) in self.locals.iter().enumerate(){
                        trace!("    [{}] {:?}", index, value);
                    }
                    match instruction {
                        Instruction::ACONST_NULL => {
                            self.stack.push(Value::Null)
                        }
                        Instruction::PUTSTATIC(index) => {
                            let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(index).expect("GIB MICH DIE FELD");
                            let (field_index, info, class_id) = self.class_and_method.class.find_field_static(field_name.as_str()).unwrap();
                            debug!("PUTSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                            let value = self.stack.pop().unwrap();
                            //let class_id = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap().id;
                            let object = vm.get_static_class_object(class_id).unwrap();
                            object.set_field(field_index, value);
                        }
                        Instruction::GETSTATIC(index) => {
                            let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(index).expect("GIB MICH DIE FELD2");
                            //let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                            //let class = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap();
                            let class = vm.get_or_resolve_class(class_name.as_str())?;
                            let (field_index, info, class_id) = class.find_field_static(field_name.as_str()).unwrap();
                            let object = vm.get_static_class_object(class_id).unwrap();
                            debug!("GETSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                            self.stack.push(object.get_field(field_index));
                        }
                        Instruction::LDC(index) => {
                            let value = self.get_constant_as_value(vm, index as u16)?;
                            self.stack.push(value);
                            debug!("LDC: {:?}", get_constant_printable(constants, index as u16))
                        }
                        Instruction::LDCW(index) => {
                            let value = self.get_constant_as_value(vm, index)?;
                            self.stack.push(value);
                            debug!("LDCW: {:?}", get_constant_printable(constants, index))
                        }
                        Instruction::LDC2W(index) => {
                            let value = self.get_constant_as_value(vm, index)?;
                            self.stack.push(value);
                            debug!("LDC2W: {}", get_constant_printable(constants, index))
                        }
                        Instruction::PUTFIELD(index) => {
                            let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(index).expect("GIB MICH DIE FELD");
                            let target_class = if class_name == self.class_and_method.class.name {
                                self.class_and_method.class
                            } else {
                                vm.get_or_resolve_class(class_name.as_str())?
                            };
                            let (field_index, info) = target_class.find_field(field_name.as_str()).unwrap();
                            debug!("PUTFIELD {}.{} {} {} {:?}", class_name, field_name, descriptor, field_index, info);
                            let value = self.stack.pop().unwrap();
                            let object = self.stack.pop().unwrap();
                            if let Value::Reference(mut obj) = object {
                                obj.set_field(field_index, value);
                                debug!("obj:{:?}", &obj);
                            } else {
                                warn!("NAO");
                            }
                        }
                        Instruction::GETFIELD(index) => {
                            let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(index).expect("GIB MICH DIE FELD2");
                            debug!("GETFIELD {}.{} {}", class_name, field_name, descriptor);
                            let target_class = if class_name == self.class_and_method.class.name {
                                self.class_and_method.class
                            } else {
                                vm.get_or_resolve_class(class_name.as_str())?
                            };
                            let (field_index, _) = target_class.find_field(field_name.as_str()).unwrap();
                            let object = self.stack.pop().unwrap();
                            if let Value::Reference(obj) = object {
                                self.stack.push(obj.get_field(field_index));
                            } else {
                                warn!("NAO");
                            }
                        }
                        Instruction::INVOKEVIRTUAL(index) => { self.execute_invoke(vm, index, InvokeKind::VIRTUAL)? }
                        Instruction::INVOKESPECIAL(index) => { self.execute_invoke(vm, index, InvokeKind::SPECIAL)? }
                        Instruction::INVOKESTATIC(index) => { self.execute_invoke(vm, index, InvokeKind::STATIC)? }
                        Instruction::INVOKEINTERFACE(index, _, _) => { self.execute_invoke(vm, index, InvokeKind::INTERFACE)? }

                        Instruction::RETURN => {
                            info!("RETURN");
                            return Ok(None);
                        }
                        Instruction::IRETURN | Instruction::LRETURN | Instruction::FRETURN | Instruction::DRETURN | Instruction::ARETURN=> {
                            //TODO check for types
                            let value = self.stack.pop();
                            info!("RETURN {:?}", value);
                            return Ok(value);
                        }
                        Instruction::NEW(index) => {
                            let class_name = self.class_and_method.get_constant_utf8(index).unwrap();
                            //let res = vm.invoke_method(class_name.as_str(), "<init>", "()V")?;
                            let new_object = vm.new_object(class_name.as_str())?;

                            debug!("NEW: {} {} {:?}", index, get_constant_printable(constants, index), &new_object);
                            self.stack.push(Value::Reference(new_object));
                        }
                        Instruction::ANEWARRAY(index) => {
                            let count = self.pop_int()?;
                            let class_name = self.class_and_method.get_constant_utf8(index).unwrap();
                            debug!("ANEWARRAY {}[{}]", class_name, count);
                            let array_content = vec![Value::Null; count as usize];
                            let array = Value::Reference(vm.new_array(1, FieldType::Object(class_name), RefCell::new(array_content))?);
                            self.stack.push(array);
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
                            let count = self.pop_int()?;
                            debug!("NEWARRAY {:?}[{}]", primitive_type, count);
                            let array_content = vec![Value::Null; count as usize];
                            let array = Value::Reference(vm.new_array(1, primitive_type, RefCell::new(array_content))?);
                            self.stack.push(array);
                        }
                        Instruction::ARRAYLENGTH => {
                            debug!("ARRAYLENGTH");
                            let popped = self.stack.pop();
                            if let Some(Value::Reference(reference)) = popped{
                                if let ReferenceType::Array(_, _, content) = &reference.reference_type{
                                    self.stack.push(Value::Integer(content.borrow().len() as i32));
                                } else {
                                    return Err(VmError::ValidationError("Expected an Array ref but found: Object ref".to_string()))
                                }
                            } else if let Some(Value::Null) = popped{
                                return Err(VmError::JavaException(JavaError::NullPointerException("Expected an array".to_string())))
                            } else {
                                return Err(VmError::ValidationError(format!("Expected an array ref but found: {:?}", &popped)))
                            }
                        }
                        Instruction::DUP => {
                            debug!("DUP");
                            let value = self.stack.pop().unwrap();
                            self.stack.push(value.clone());
                            self.stack.push(value);
                        }
                        Instruction::DUPX1 => {
                            debug!("DUPX1");
                            let value = self.stack.pop().unwrap();
                            self.stack.insert(self.stack.len() - 1, value.clone());
                            self.stack.push(value);
                        }
                        Instruction::DUP2 => {
                            let optional_value1 = self.stack.pop();
                            let optional_value2 = self.stack.pop();
                            if let (Some(value1), Some(value2)) = (optional_value1.clone(), optional_value2) {
                                debug!("DUP2 Comp type 1");
                                self.stack.push(value2.clone());
                                self.stack.push(value1.clone());
                                self.stack.push(value2);
                                self.stack.push(value1);
                            } else if let Some(value) = optional_value1{
                                debug!("DUP2 Comp type 2");
                                self.stack.push(value.clone());
                                self.stack.push(value);
                            }
                        }
                        Instruction::POP => {
                            debug!("POP");
                            if self.stack.pop().is_none(){
                                warn!("Expected a value to pop");
                            }
                        }
                        Instruction::IF_ACMPNE(offset) => {
                            let o1 = self.stack.pop().unwrap();
                            let o2 = self.stack.pop().unwrap();
                            match (o1, o2) {
                                (Value::Reference(obj1), Value::Reference(obj2)) => {
                                    debug!("IF_ACMPNE {:?} != {:?}?", obj1.id, obj2.id);
                                    if obj1.id != obj2.id {
                                        self.pc.0 = offset
                                    }
                                }
                                /*(Value::Reference(_), Value::Null) | (Value::Null, Value::Reference(_)) => {
                                    debug!("IF_ACMPNE One is null");
                                    self.pc.0 = offset
                                }*/
                                _ => {}
                            };
                        }
                        Instruction::IF_ACMPEQ(offset) => {
                            let o1 = self.stack.pop().unwrap();
                            let o2 = self.stack.pop().unwrap();
                            match (o1, o2) {
                                (Value::Reference(obj1), Value::Reference(obj2)) => {
                                    debug!("IF_ACMPEQ {:?} == {:?}?", obj1.id, obj2.id);
                                    if obj1.id == obj2.id {
                                        self.pc.0 = offset
                                    }
                                }
                                (Value::Null, Value::Null) => {
                                    debug!("IF_ACMPEQ Both Null");
                                    self.pc.0 = offset
                                }
                                _ => {}
                            };
                        }
                        Instruction::IF_ICMPNE(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 != val2) }
                        Instruction::IF_ICMPGT(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 >  val2) }
                        Instruction::IF_ICMPGE(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 >= val2) }
                        Instruction::IF_ICMPEQ(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 == val2) }
                        Instruction::IF_ICMPLE(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 <= val2) }
                        Instruction::IF_ICMPLT(offset) => { self.execute_i_cmp(offset, |val1, val2| val1 <  val2) }
                        Instruction::IFNONNULL(offset) => {
                            let reference = self.stack.pop().unwrap();
                            match reference {
                                Value::Null => {debug!("IFNONNULL is NULL");}
                                Value::Reference(_) => {debug!("IFNONNULL is reference"); self.pc.0 = offset}
                                _ => {warn!("IFNONNULL {:?} is this valid?", reference.clone())}
                            }
                        }
                        Instruction::IFNULL(offset) => {
                            let reference = self.stack.pop().unwrap();
                            match reference {
                                Value::Null => {debug!("IFNULL is NULL"); self.pc.0 = offset}
                                Value::Reference(_) => {debug!("IFNULL is reference");}
                                _ => {warn!("IFNULL {:?} is this valid?", reference.clone())}
                            }
                        }
                        Instruction::IFGT(offset) => { self.execute_cmp(offset, |value| value >  0) }
                        Instruction::IFGE(offset) => { self.execute_cmp(offset, |value| value >= 0) }
                        Instruction::IFEQ(offset) => { self.execute_cmp(offset, |value| value == 0) }
                        Instruction::IFLE(offset) => { self.execute_cmp(offset, |value| value <= 0) }
                        Instruction::IFLT(offset) => { self.execute_cmp(offset, |value| value <  0) }
                        Instruction::IFNE(offset) => { self.execute_cmp(offset, |value| value != 0) }
                        Instruction::FCMPG | Instruction::FCMPL => {
                            if let (Some(Value::Float(value2)), Some(Value::Float(value1))) = (self.stack.pop(), self.stack.pop()){
                                debug!("FCMP");
                                if value1 > value2{
                                    self.stack.push(Value::Integer(1))
                                } else if value1 == value2{
                                    self.stack.push(Value::Integer(0))
                                } else if value1 < value2{
                                    self.stack.push(Value::Integer(-1))
                                } else if value1.is_nan() || value2.is_nan(){
                                    if instruction == Instruction::FCMPG{
                                        self.stack.push(Value::Integer(1))
                                    } else {
                                        self.stack.push(Value::Integer(-1))
                                    }
                                }
                            }
                        }
                        Instruction::DCMPG | Instruction::DCMPL => {
                            if let (Some(Value::Double(value2)), Some(Value::Double(value1))) = (self.stack.pop(), self.stack.pop()){
                                debug!("DCMP");
                                if value1 > value2{
                                    self.stack.push(Value::Integer(1))
                                } else if value1 == value2{
                                    self.stack.push(Value::Integer(0))
                                } else if value1 < value2{
                                    self.stack.push(Value::Integer(-1))
                                } else if value1.is_nan() || value2.is_nan(){
                                    if instruction == Instruction::DCMPG{
                                        self.stack.push(Value::Integer(1))
                                    } else {
                                        self.stack.push(Value::Integer(-1))
                                    }
                                }
                            }
                        }
                        Instruction::LCMP => {
                            if let (Some(Value::Long(value2)), Some(Value::Long(value1))) = (self.stack.pop(), self.stack.pop()) {
                                debug!("LCMP");
                                if value1 > value2 {
                                    self.stack.push(Value::Integer(1))
                                } else if value1 == value2 {
                                    self.stack.push(Value::Integer(0))
                                } else if value1 < value2 {
                                    self.stack.push(Value::Integer(-1))
                                }
                            }
                        }
                        Instruction::GOTO(offset) => {
                            debug!("GOTO {}", offset);
                            self.pc.0 = offset
                        }
                        Instruction::LOOKUPSWITCH(default, pair_stream) => {
                            let popped = self.pop_int()?;
                            debug!("LOOKUPSWITCH: {}", popped);
                            let mut use_default = true;
                            for chunk in pair_stream.chunks(2){
                                let (int_match, offset) = (chunk[0], chunk[1]);
                                if int_match == popped{
                                    self.pc.0 = offset as u16;
                                    use_default = false;
                                    break;
                                }
                            }
                            if use_default{
                                self.pc.0 = default as u16;
                            }
                        }
                        Instruction::ISTORE(index) => { self.execute_istore(index as usize)? }
                        Instruction::ISTORE0 => { self.execute_istore(0)? }
                        Instruction::ISTORE1 => { self.execute_istore(1)? }
                        Instruction::ISTORE2 => { self.execute_istore(2)? }
                        Instruction::ISTORE3 => { self.execute_istore(3)? }

                        Instruction::LSTORE0 => { self.execute_lstore(0)? }
                        Instruction::LSTORE1 => { self.execute_lstore(1)? }
                        Instruction::LSTORE2 => { self.execute_lstore(2)? }
                        Instruction::LSTORE3 => { self.execute_lstore(3)? }

                        Instruction::ASTORE(index) => { self.execute_astore(index as usize)? }
                        Instruction::ASTORE0 => { self.execute_astore(0)? }
                        Instruction::ASTORE1 => { self.execute_astore(1)? }
                        Instruction::ASTORE2 => { self.execute_astore(2)? }
                        Instruction::ASTORE3 => { self.execute_astore(3)? }
                        Instruction::IASTORE | Instruction::AASTORE | Instruction::CASTORE | Instruction::BASTORE => {
                            //TODO validate type of value to fit instruction
                            let value = self.stack.pop().unwrap();
                            let index = self.pop_int()?;
                            let popped = self.stack.pop().unwrap();
                            debug!("XASTORE: {:?}", popped);
                            if let Value::Reference(array_ref) = popped{
                                array_ref.set_element(index as usize, value);
                            }
                        }
                        Instruction::ICONSTM1 => { self.execute_iconst(-1) }
                        Instruction::ICONST0 => { self.execute_iconst(0) }
                        Instruction::ICONST1 => { self.execute_iconst(1) }
                        Instruction::ICONST2 => { self.execute_iconst(2) }
                        Instruction::ICONST3 => { self.execute_iconst(3) }
                        Instruction::ICONST4 => { self.execute_iconst(4) }
                        Instruction::ICONST5 => { self.execute_iconst(5) }

                        Instruction::LCONST0 => { self.execute_lconst(0) }
                        Instruction::LCONST1 => { self.execute_lconst(1) }

                        Instruction::FCONST0 => { self.execute_fconst(0) }
                        Instruction::FCONST1 => { self.execute_fconst(1) }
                        Instruction::FCONST2 => { self.execute_fconst(2) }

                        Instruction::DCONST0 => { self.execute_dconst(0) }
                        Instruction::DCONST1 => { self.execute_dconst(1) }

                        Instruction::ILOAD(index) => { self.execute_iload(index as usize)? }
                        Instruction::ILOAD0 => { self.execute_iload(0)? }
                        Instruction::ILOAD1 => { self.execute_iload(1)? }
                        Instruction::ILOAD2 => { self.execute_iload(2)? }
                        Instruction::ILOAD3 => { self.execute_iload(3)? }

                        Instruction::LLOAD(index) => { self.execute_lload(index as usize)? }
                        Instruction::LLOAD0 => { self.execute_lload(0)? }
                        Instruction::LLOAD1 => { self.execute_lload(1)? }
                        Instruction::LLOAD2 => { self.execute_lload(2)? }
                        Instruction::LLOAD3 => { self.execute_lload(3)? }

                        Instruction::FLOAD0 => { self.execute_fload(0)? }
                        Instruction::FLOAD1 => { self.execute_fload(1)? }
                        Instruction::FLOAD2 => { self.execute_fload(2)? }
                        Instruction::FLOAD3 => { self.execute_fload(3)? }

                        Instruction::DLOAD0 => { self.execute_dload(0)? }
                        Instruction::DLOAD1 => { self.execute_dload(1)? }
                        Instruction::DLOAD2 => { self.execute_dload(2)? }
                        Instruction::DLOAD3 => { self.execute_dload(3)? }

                        Instruction::ALOAD(index) => { self.execute_aload(index as usize)? }
                        Instruction::ALOAD0 => { self.execute_aload(0)? }
                        Instruction::ALOAD1 => { self.execute_aload(1)? }
                        Instruction::ALOAD2 => { self.execute_aload(2)? }
                        Instruction::ALOAD3 => { self.execute_aload(3)? }
                        Instruction::BIPUSH(value) => {
                            debug!("BIPUSH {:?}", value);
                            self.stack.push(Value::Integer(value as i32))
                        }
                        Instruction::SIPUSH(value) => {
                            debug!("SIPUSH {:?}", value);
                            self.stack.push(Value::Integer(value as i32))
                        }
                        Instruction::IINC(index, amount) => {
                            if let Some(Value::Integer(value)) = &self.locals.get(index as usize){
                                self.locals[index as usize] = Value::Integer(value + amount as i32)
                            }
                        }
                        //TODO add type validation
                        Instruction::AALOAD | Instruction::IALOAD | Instruction::BALOAD | Instruction::CALOAD => {
                            let index = self.pop_int()?;
                            if let Some(Value::Reference(array_ref)) = self.stack.pop(){
                                self.stack.push(array_ref.get_element(index as usize));
                            }
                        }
                        Instruction::ISUB => { self.execute_i_arithmetic(|val1, val2| Ok(val1.wrapping_sub(val2)))? }
                        Instruction::IMUL => { self.execute_i_arithmetic(|val1, val2| Ok(val1.wrapping_mul(val2)))? }
                        Instruction::IADD => { self.execute_i_arithmetic(|val1, val2| Ok(val1.wrapping_add(val2)))? }
                        //TODO check if val2 is zero -> error
                        Instruction::IREM => { self.execute_i_arithmetic(|val1, val2| Ok(val1.wrapping_rem(val2)))? }
                        Instruction::IDIV => { self.execute_i_arithmetic(|val1, val2| Ok(val1.wrapping_div(val2)))? }
                        Instruction::INEG => {
                            let value = self.pop_int()?;
                            debug!("INEG");
                            self.stack.push(Value::Integer(-value))
                        }
                        Instruction::IXOR => { self.execute_i_arithmetic(|val1, val2| Ok(val1 ^ val2))? }
                        Instruction::IAND => { self.execute_i_arithmetic(|val1, val2| Ok(val1 & val2))? }
                        Instruction::IOR  => { self.execute_i_arithmetic(|val1, val2| Ok(val1 | val2))? }
                        Instruction::ISHL => { self.execute_i_arithmetic(|val1, val2| Ok(val1 << (val2 & 0x1f)))? }
                        Instruction::ISHR => { self.execute_i_arithmetic(|val1, val2| Ok(val1 >> (val2 & 0x1f)))? }
                        Instruction::IUSHR => { self.execute_i_arithmetic(|val1, val2| {
                            if val1 > 0{
                                Ok(val1 >> (val2 & 0x1f))
                            } else {
                                Ok(((val1 as u32) >> (val2 & 0x1f)) as i32)
                            }
                        })?}
                        Instruction::LADD => { self.execute_j_arithmetic(|val1, val2| Ok(val1.wrapping_add(val2)))? }
                        Instruction::LAND => { self.execute_j_arithmetic(|val1, val2| Ok(val1 & val2))? },
                        Instruction::LUSHR => { self.execute_ji_arithmetic(|val1, val2| {
                            if val1 > 0{
                                Ok(val1 >> (val2 & 0x1f))
                            } else {
                                Ok(((val1 as u64) >> (val2 & 0x1f)) as i64)
                            }
                        })?}
                        Instruction::FMUL => { self.execute_f_arithmetic(|val1, val2| Ok(val1 * val2))? }
                        Instruction::FDIV => { self.execute_f_arithmetic(
                            |val1, val2|
                            if val2 != 0.0 {
                                Ok(val1 / val2)
                            } else {
                                Err(VmError::JavaException(JavaError::DivisionByZero))
                            }
                        )?}
                        Instruction::DADD => { self.execute_d_arithmetic(|val1, val2| Ok(val1 + val2))? }
                        Instruction::DDIV => { self.execute_d_arithmetic(
                            |val1, val2|
                            if val2 != 0.0 {
                                Ok(val1 / val2)
                            } else {
                                Err(VmError::JavaException(JavaError::DivisionByZero))
                            }
                        )?}
                        Instruction::I2B => {
                            let value = self.stack.pop().unwrap();
                            debug!("I2B");
                            if let Value::Integer(int) = value {
                                self.stack.push(Value::Integer(int));
                            } else {
                                warn!("I2B Conversion failed, because {value:?} is not of type Int")
                            }
                        }
                        Instruction::I2L => {
                            let value = self.stack.pop().unwrap();
                            debug!("I2L");
                            if let Value::Integer(long) = value {
                                self.stack.push(Value::Long(long as i64));
                            } else {
                                warn!("I2L Conversion failed, because {value:?} is not of type Int")
                            }
                        }
                        Instruction::I2F => {
                            let value = self.stack.pop().unwrap();
                            debug!("I2F");
                            if let Value::Integer(int) = value {
                                self.stack.push(Value::Float(int as f32));
                            } else {
                                warn!("I2F Conversion failed, because {value:?} is not of type Int")
                            }
                        }
                        Instruction::I2D => {
                            let value = self.stack.pop().unwrap();
                            debug!("I2D");
                            if let Value::Integer(int) = value {
                                self.stack.push(Value::Double(int as f64));
                            } else {
                                warn!("I2D Conversion failed, because {value:?} is not of type Int")
                            }
                        }
                        Instruction::I2C => {
                            let value = self.stack.pop().unwrap();
                            debug!("I2C");
                            if let Value::Integer(int) = value {
                                self.stack.push(Value::Integer(int));
                            } else {
                                warn!("I2C Conversion failed, because {value:?} is not of type Int")
                            }
                        }
                        Instruction::L2I => {
                            let value = self.stack.pop().unwrap();
                            debug!("L2I");
                            if let Value::Long(long) = value {
                                self.stack.push(Value::Integer(long as i32));
                            } else {
                                warn!("L2I Conversion failed, because {value:?} is not of type Long")
                            }
                        }
                        Instruction::F2I => {
                            let value = self.stack.pop().unwrap();
                            debug!("F2I");
                            if let Value::Float(float) = value {
                                self.stack.push(Value::Integer(float as i32));
                            } else {
                                warn!("F2I Conversion failed, because {value:?} is not of type Float")
                            }
                        }
                        Instruction::D2I => {
                            let value = self.stack.pop().unwrap();
                            debug!("D2I");
                            if let Value::Double(double) = value {
                                self.stack.push(Value::Integer(double as i32));
                            } else {
                                warn!("D2I Conversion failed, because {value:?} is not of type Double")
                            }
                        }
                        Instruction::MONITORENTER => {
                            if let Some(Value::Reference(_)) = self.stack.pop(){
                                debug!("MONITORENTER")
                            } else {
                                warn!("No object to lock")
                            }
                        }
                        Instruction::MONITOREXIT => {
                            if let Some(Value::Reference(_)) = self.stack.pop(){
                                debug!("MONITOREXIT")
                            } else {
                                warn!("No object to lock")
                            }
                        }
                        Instruction::CHECKCAST(constant_index) => {
                            //TODO
                            debug!("CHECKCAST {}", get_constant_printable(constants, constant_index));
                        }
                        Instruction::INSTANCEOF(constant_index) => {
                            debug!("INSTANCEOF {}", get_constant_printable(constants, constant_index));
                            self.stack.push(Value::Integer(1));
                        }
                        Instruction::ATHROW => {
                            if let Some(Value::Reference(error)) = self.stack.pop(){
                                let string_value = error.get_field(2);
                                let string = vm.extract_string_from_object(&string_value)?;
                                let exception_name = vm.class_manager.find_class_by_id(error.class_id).unwrap().name.clone();
                                return Err(VmError::JavaException(JavaError::JavaExceptionThrown(exception_name, string)));
                            }
                            return Err(VmError::JavaException(JavaError::JavaExceptionThrown("JavaException".to_string(), "Unknown".to_string())));
                        }
                        _ => { unimplemented!("Instruction {:?} not executable (stack={:?})", instruction, &self.stack) }
                    }
                } else {
                    break;
                }
            }
        }
        if self.class_and_method.method.is_abstract(){
            Err(VmError::MethodCallError(format!("abstract method {}", self.class_and_method.method.name)))
        } else {
            Err(VmError::MethodCallError(format!("{}", self.class_and_method.method.name)))
        }
    }

    fn execute_istore(&mut self, index: usize) -> Result<(), VmError>{
        let popped = self.stack.pop();
        if let Some(Value::Integer(value)) = popped{
            debug!("ISTORE{} {:?}", index, value);
            self.locals[index] = popped.unwrap();
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("ISTORE{} failed, because stack[{}] was {:?} and not Integer", index, index, popped)))
        }
    }
    fn execute_lstore(&mut self, index: usize) -> Result<(), VmError>{
        let popped = self.stack.pop();
        if let Some(Value::Long(value)) = popped{
            debug!("LSTORE{} {:?}", index, value);
            self.locals[index] = popped.unwrap();
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("LSTORE{} failed, because stack[{}] was {:?} and not Long", index, index, popped)))
        }
    }

    fn execute_astore(&mut self, index: usize) -> Result<(), VmError>{
        //TODO validation
        let popped = self.stack.pop();
        if let Some(value) = popped{
            debug!("ASTORE{} {:?}", index, value);
            self.locals[index] = value;
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("ASTORE{} failed, because stack[{}] was {:?} and not Object", index, index, popped)))
        }
    }

    fn execute_iload(&mut self, index: usize) -> Result<(), VmError>{
        let local = self.locals.get(index);
        if let Some(Value::Integer(value)) = local{
            self.stack.push(Value::Integer(*value));
            debug!("ILOAD{} {:?}", index, value);
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("ILOAD{} failed, because locals[{}] was {:?} and not Integer", index, index, local)))
        }
    }

    fn execute_lload(&mut self, index: usize) -> Result<(), VmError>{
        let local = self.locals.get(index);
        if let Some(Value::Long(value)) = local{
            self.stack.push(Value::Long(*value));
            debug!("LLOAD{} {:?}", index, value);
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("LLOAD{} failed, because locals[{}] was {:?} and not Long", index, index, local)))
        }
    }

    fn execute_fload(&mut self, index: usize) -> Result<(), VmError>{
        let local = self.locals.get(index);
        if let Some(Value::Float(value)) = local{
            self.stack.push(Value::Float(*value));
            debug!("FLOAD{} {:?}", index, value);
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("FLOAD{} failed, because locals[{}] was {:?} and not Float", index, index, local)))
        }
    }

    fn execute_dload(&mut self, index: usize) -> Result<(), VmError>{
        let local = self.locals.get(index);
        if let Some(Value::Double(value)) = local{
            self.stack.push(Value::Double(*value));
            debug!("DLOAD{} {:?}", index, value);
            Ok(())
        } else {
            Err(VmError::ValidationError(format!("DLOAD{} failed, because locals[{}] was {:?} and not Double", index, index, local)))
        }
    }

    fn execute_aload(&mut self, index: usize) -> Result<(), VmError>{
        let popped = self.locals.get(index).unwrap();
        match popped {
            Value::Reference(reference) => {
                self.stack.push(Value::Reference(reference));
                debug!("ALOAD{} {:?}", index, reference);
            }
            Value::Null => {
                self.stack.push(Value::Null);
                debug!("ALOAD{} (loaded null)", index);
            }
            _ => return Err(VmError::ValidationError(format!("ALOAD{} failed", index)))
        }
        Ok(())
    }

    fn execute_iconst(&mut self, value: isize){
        debug!("ICONST {:?}", value);
        self.stack.push(Value::Integer(value as i32))
    }

    fn execute_lconst(&mut self, value: isize){
        debug!("LCONST {:?}", value);
        self.stack.push(Value::Long(value as i64))
    }

    fn execute_fconst(&mut self, value: usize){
        debug!("FCONST {:?}", value);
        self.stack.push(Value::Float(value as f32))
    }

    fn execute_dconst(&mut self, value: usize){
        debug!("DCONST {:?}", value);
        self.stack.push(Value::Double(value as f64))
    }

    fn execute_i_arithmetic<F: FnOnce(i32, i32) -> Result<i32, VmError>>(&mut self, f: F) -> Result<(), VmError>{
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Integer(val1)), Some(Value::Integer(val2))) = (value1, value2){
            let res = f(val1, val2)?;
            debug!("Integer ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Integer(res));
            Ok(())
        } else {
            warn!("dat sin nich zwee ints to keck");
            Err(VmError::ValidationError("Expected two ints".to_string()))
        }
    }

    fn execute_ji_arithmetic<F: FnOnce(i64, i32) -> Result<i64, VmError>>(&mut self, f: F) -> Result<(), VmError>{
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Long(val1)), Some(Value::Integer(val2))) = (value1, value2){
            let res = f(val1, val2)?;
            debug!("LongInt ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Long(res));
            Ok(())
        } else {
            warn!("dat sin nich eene long und eene int du keck");
            Err(VmError::ValidationError("Expected an int and a long".to_string()))
        }
    }

    fn execute_j_arithmetic<F: FnOnce(i64, i64) -> Result<i64, VmError>>(&mut self, f: F) -> Result<(), VmError>{
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Long(val1)), Some(Value::Long(val2))) = (value1, value2){
            let res = f(val1, val2)?;
            debug!("Long ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Long(res));
            Ok(())
        } else {
            warn!("dat sin nich zwee longse to keck");
            Err(VmError::ValidationError("Expected two longs".to_string()))
        }
    }

    fn execute_f_arithmetic<F: FnOnce(f32, f32) -> Result<f32, VmError>>(&mut self, f: F) -> Result<(), VmError>{
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Float(val1)), Some(Value::Float(val2))) = (value1, value2){
            let res = f(val1, val2)?;
            debug!("Float ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Float(res));
            Ok(())
        } else {
            warn!("dat sin nich zwee floatse to keck");
            Err(VmError::ValidationError("Expected two floats".to_string()))
        }
    }

    fn execute_d_arithmetic<F: FnOnce(f64, f64) -> Result<f64, VmError>>(&mut self, f: F) -> Result<(), VmError>{
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Double(val1)), Some(Value::Double(val2))) = (value1, value2){
            let res = f(val1, val2)?;
            debug!("Double ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Double(res));
            Ok(())
        } else {
            warn!("dat sin nich zwee doppelte to keck");
            Err(VmError::ValidationError("Expected two doubles".to_string()))
        }
    }

    fn execute_i_cmp<F: FnOnce(i32, i32) -> bool>(&mut self, offset: u16, f: F){
        let val2 = self.pop_int().unwrap();
        let val1 = self.pop_int().unwrap();
        let jump = f(val1, val2);
        debug!("ICMP: {}&{}={}", val1, val2, jump);
        if jump{
            self.pc.0 = offset
        }
    }

    fn execute_cmp<F: FnOnce(i32) -> bool>(&mut self, offset: u16, cmp: F){
        let value = self.pop_int().unwrap();
        if cmp(value){
            self.pc.0 = offset;
        }
    }

    fn execute_invoke(&mut self, vm: &mut VM<'a>, index: u16, kind: InvokeKind) -> Result<(), VmError> {
        let (class_name, method_name, descriptor) = self.class_and_method.get_constant_method_info_descriptor(index).expect("GIB MICH DIE METHODE");
        let args_count = MethodDescriptor::new(descriptor.clone()).args.len();
        let mut args = Vec::new();
        for _ in 0..args_count{
            let popped = self.stack.pop().unwrap();
            match popped {
                Value::Long(_) | Value::Double(_) => {args.insert(0, Value::Uninitialized)}
                _ => {}
            }
            args.insert(0, popped);
        }
        if class_name.starts_with("["){
            let receiver = self.stack.pop().unwrap();
            return if let Value::Reference(array_ref) = receiver{
                if method_name == "clone"{
                    if let ReferenceType::Array(dims, field_type, content) = &array_ref.reference_type{
                        debug!("Cloning array: {:?}", receiver);
                        self.stack.push(Value::Reference(vm.new_array(*dims, field_type.clone(), content.clone())?));
                        Ok(())
                    } else {
                        Err(VmError::ValidationError("Expected array to be cloned".to_string()))
                    }
                } else {
                    Err(VmError::MethodCallError(format!("Method {} not supported for arrays", method_name)))
                }
            } else {
                Err(VmError::ValidationError("Expected array as receiver".to_string()))
            };
        }

        trace!("loading class to execute on: '{}'", class_name.as_str());
        let class = vm.get_or_resolve_class(class_name.as_str())?;
        trace!("finished loading class to execute on: '{}'", class_name.as_str());
        let class_and_method = match kind {
            InvokeKind::STATIC | InvokeKind::SPECIAL => {
                class.find_method(method_name.as_str(), descriptor.as_str()).map(|method| ClassAndMethod {class, method}).unwrap()
            }
            InvokeKind::VIRTUAL | InvokeKind::INTERFACE => {
                Self::get_method_virtual(class, method_name.as_str(), descriptor.as_str())?
            }
        };
        let receiver = if class_and_method.method.is_static(){
            None
        } else {
            let popped = self.stack.pop();
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
                        let resolved_method = Self::get_method_virtual(receiver_class, class_and_method.method.name.as_str(), class_and_method.method.descriptor.as_str())?;
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

        trace!("STATUS of '{}' before invoke: ", self.class_and_method.method.name);
        trace!("stack=");
        for (index, value) in self.stack.iter().enumerate(){
            trace!("    [{}] {:?}", index, value);
        }
        trace!("locals=");
        for (index, value) in self.locals.iter().enumerate(){
            trace!("    [{}] {:?}", index, value);
        }
        debug!("INVOKE{:?}: {}{} on {:?}", kind, method_name, descriptor, receiver);
        let res = vm.invoke(class_and_method, receiver, args)?;
        if res.is_some(){
            self.stack.push(res.unwrap())
        }
        Ok(())
    }

    fn get_method_virtual(class: ClassRef<'a>, method_name: &str, descriptor: &str) -> Result<ClassAndMethod<'a>, VmError>{
        let mut current_class = class;
        loop {
            if let Some(method) = current_class.find_method(method_name, descriptor){
                return Ok(ClassAndMethod{class: current_class, method});
            }
            if let Some(super_class) = current_class.superclass{
                current_class = super_class
            } else {
                return Err(VmError::JavaException(JavaError::MethodNotFoundException(method_name.to_string() + descriptor)));
            }
        }
    }

    fn pop_int(&mut self) -> Result<i32, VmError>{
        let popped = self.stack.pop();
        if let Some(Value::Integer(value)) = popped{
            return Ok(value)
        }
        Err(VmError::ValidationError(format!("Expected Integer to pop but found {:?}", popped)))
    }

    fn get_constant_as_value(&mut self, vm: &mut VM<'a>, index: u16) -> Result<Value<'a>, VmError>{
        let constant_value = self.class_and_method.class.get_constant(index).unwrap();
        let value = match constant_value {
            ConstantPoolEntry::Integer(value) => Value::Integer(value),
            ConstantPoolEntry::Long(value) => Value::Long(value),
            ConstantPoolEntry::Float(value) => Value::Float(value),
            ConstantPoolEntry::Double(value) => Value::Double(value),
            ConstantPoolEntry::String(string_index) => {
                if let Some(ConstantPoolEntry::Utf8(string)) = self.class_and_method.class.get_constant(string_index){
                    let string_object = vm.new_string_object(string)?;
                    Value::Reference(string_object)
                } else {
                    //return Err(ValidationError("Expected string".to_string()));
                    warn!("expected but didnt find string object");
                    Value::Null
                }
            }
            ConstantPoolEntry::Class(name_index) => {
                if let Some(ConstantPoolEntry::Utf8(string)) = self.class_and_method.class.get_constant(name_index){
                    let class_object = vm.new_class_object(string)?;
                    Value::Reference(class_object)
                } else {
                    warn!("expected but didnt find string object");
                    Value::Null
                }
            }
            _ => unimplemented!("Constant of type {constant_value:?} cannot be converted to a value")
        };
        Ok(value)
    }
}

impl Debug for CallFrame<'_>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Method: {}.{}{} at {:?}", self.class_and_method.class.name, self.class_and_method.method.name, self.class_and_method.method.descriptor.as_str(), self.pc)
    }
}

#[derive(Debug, PartialEq)]
enum InvokeKind{
    STATIC,
    SPECIAL,
    VIRTUAL,
    INTERFACE,
}