use crate::attribute::ProgramCounter;
use crate::bytecode::Instruction;
use crate::get_constant_printable;
use crate::method_info::MethodDescriptor;
use crate::vm::{VM, VmError};
use crate::vm::class::ClassAndMethod;
use crate::vm::value::Value;

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
            println!();
            println!("{:?}", &code.code);

            loop{
                let instruction = code.code.get(self.pc.0 as usize).unwrap();
                println!("stack={:?}, locals={:?}", &self.stack, &self.locals);
                match instruction {
                    Instruction::PUTSTATIC(index) => {
                        let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD");
                        let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                        println!("PUTSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                        let value = self.stack.pop().unwrap();
                        let class_id = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap().id;
                        let object = vm.get_static_class_object(class_id).unwrap();
                        object.set_field(field_index, value);
                    }
                    Instruction::GETSTATIC(index) => {
                        let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD2");
                        //let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                        //let class = vm.class_manager.find_class_by_name(class_name.as_str()).unwrap();
                        let class = vm.get_or_resolve_class(class_name.as_str())?;
                        let (field_index, info) = class.find_field(field_name.as_str()).unwrap();
                        let object = vm.get_static_class_object(class.id).unwrap();
                        println!("GETSTATIC {} {} {} {:?}", field_name, descriptor, field_index, info);
                        self.stack.push(object.get_field(field_index));
                    }
                    Instruction::LDC(index) => {
                        self.stack.push(self.class_and_method.get_constant_as_value(*index as u16));
                        println!("LDC: {}", get_constant_printable(constants, *index as u16))
                    }
                    Instruction::LDC2W(index) => {
                        self.stack.push(self.class_and_method.get_constant_as_value(*index));
                        println!("LDC2W: {}", get_constant_printable(constants, *index))
                    }
                    Instruction::PUTFIELD(index) => {
                        let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD");
                        let (field_index, info) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                        println!("PUTFIELD {} {} {} {:?}", field_name, descriptor, field_index, info);
                        let value = self.stack.pop().unwrap();
                        let object = self.stack.pop().unwrap();
                        if let Value::Object(mut obj) = object{
                            obj.set_field(field_index, value);
                            println!("obj:{:?}", &obj);
                        } else {
                            println!("NAO");
                        }
                    }
                    Instruction::GETFIELD(index) => {
                        let (class_name, field_name, descriptor) = self.class_and_method.get_constant_field_info_descriptor(*index).expect("GIB MICH DIE FELD2");
                        println!("GETFIELD {} {}", field_name, descriptor);
                        let (field_index, _) = self.class_and_method.class.find_field(field_name.as_str()).unwrap();
                        let object = self.stack.pop().unwrap();
                        if let Value::Object(obj) = object{
                            self.stack.push(obj.get_field(field_index));
                        } else {
                            println!("NAO");
                        }
                    }
                    Instruction::INVOKEVIRTUAL(index) => {self.execute_invoke(vm, *index, InvokeKind::VIRTUAL)?}
                    Instruction::INVOKESPECIAL(index) => {self.execute_invoke(vm, *index, InvokeKind::SPECIAL)?}
                    Instruction::INVOKESTATIC(index) => {self.execute_invoke(vm, *index, InvokeKind::STATIC)?}

                    Instruction::RETURN => {
                        println!("RETURN");
                        return Ok(None);
                    }
                    Instruction::IRETURN => {
                        let value = self.stack.pop();
                        println!("RETURN {:?}", value);
                        return Ok(value);
                    }
                    Instruction::NEW(index) => {
                        let i = *index;
                        let class_name = self.class_and_method.get_constant_utf8(i).unwrap();
                        //let res = vm.invoke_method(class_name.as_str(), "<init>", "()V")?;
                        let new_object = vm.new_object(class_name.as_str())?;

                        println!("NEW: {} {}", index, get_constant_printable(constants, i));
                        self.stack.push(Value::Object(new_object));
                    }
                    Instruction::DUP => {
                        println!("DUP");
                        let value = self.stack.pop().unwrap();
                        self.stack.push(value.clone());
                        self.stack.push(value);
                    }
                    Instruction::IF_ACMPNE(offset) => {
                        let o1 = self.stack.pop().unwrap();
                        let o2 = self.stack.pop().unwrap();
                        match (o1, o2) {
                            (Value::Object(obj1), Value::Object(obj2)) => {
                                if obj1.id != obj2.id{
                                    self.pc.0 += offset-1
                                }
                            }
                            _ => {}
                        };
                    }
                    Instruction::GOTO(offset) => {
                        self.pc.0 += offset - 1
                    }
                    Instruction::ISTORE(index) => {self.execute_istore(*index as usize)}
                    Instruction::ISTORE0 => {self.execute_istore(0)}
                    Instruction::ISTORE1 => {self.execute_istore(1)}
                    Instruction::ISTORE2 => {self.execute_istore(2)}
                    Instruction::ISTORE3 => {self.execute_istore(3)}

                    Instruction::ASTORE1 => {self.execute_astore(1)}
                    Instruction::ASTORE2 => {self.execute_astore(2)}
                    Instruction::ICONST0 => {self.execute_iconst(0)}
                    Instruction::ICONST1 => {self.execute_iconst(1)}
                    Instruction::ICONST5 => {self.execute_iconst(5)}
                    Instruction::ILOAD(index) => {self.execute_iload(*index as usize)}
                    Instruction::ILOAD1 => {self.execute_iload(1)}
                    Instruction::ILOAD2 => {self.execute_iload(2)}
                    Instruction::ALOAD0 => {self.execute_aload(0)}
                    Instruction::ALOAD1 => {self.execute_aload(1)}
                    Instruction::ALOAD2 => {self.execute_aload(2)}
                    Instruction::BIPUSH(value) => {
                        println!("BIPUSH {:?}", value);
                        self.stack.push(Value::Integer(*value as i32))
                    }
                    Instruction::ISUB => {self.execute_i_arithmetic(|val1, val2| val1 - val2)}
                    Instruction::IMUL => {self.execute_i_arithmetic(|val1, val2| val1 * val2)}
                    Instruction::IADD => {self.execute_i_arithmetic(|val1, val2| val1 + val2)}
                    Instruction::D2I => {
                        let value = self.stack.pop().unwrap();
                        println!("D2I");
                        if let Value::Double(double) = value{
                            self.stack.push(Value::Integer(double as i32));
                        } else {
                            eprintln!("D2I Conversion failed, because {value:?} is not of type Double")
                        }
                    }
                    _ => {unimplemented!("Instruction {:?} not executable (stack={:?})", instruction, &self.stack)}
                }
                self.pc.0 += 1
            }
        }
        Err(VmError::MethodCallError(self.class_and_method.method.name.to_string()))
    }

    fn execute_istore(&mut self, index: usize){
        let value = self.stack.pop().expect("LECK MICH DOCH");
        println!("ISTORE{} {:?}", index, value);
        self.locals.insert(index, value);
    }

    fn execute_astore(&mut self, index: usize){
        let value = self.stack.pop().expect("LECK MICH DOCH2");
        println!("ASTORE{} {:?}", index, value);
        self.locals.insert(index, value);
    }

    fn execute_iload(&mut self, index: usize){
        if let Value::Integer(value) = self.locals.get(index).unwrap(){
            self.stack.push(Value::Integer(*value));
            println!("ILOAD{} {:?}", index, value);
        }
    }

    fn execute_aload(&mut self, index: usize){
        if let Value::Object(value) = self.locals.get(index).unwrap(){
            self.stack.push(Value::Object(value));
            println!("ALOAD{} {:?}", index, value);
        }
    }

    fn execute_iconst(&mut self, value: usize){
        println!("ICONST {:?}", value);
        self.stack.push(Value::Integer(value as i32))
    }

    fn execute_i_arithmetic<F: FnOnce(i32, i32) -> i32>(&mut self, f: F){
        let value2 = self.stack.pop();
        let value1 = self.stack.pop();
        if let (Some(Value::Integer(val1)), Some(Value::Integer(val2))) = (value1, value2){
            let res = f(val1, val2);
            println!("Integer ARITHMETIC {}&{}={}", val1, val2, res);
            self.stack.push(Value::Integer(res))
        } else {
            println!("dat sin nich zwee ints to keck");
        }
    }

    fn execute_invoke(&mut self, vm: &mut VM<'a>, index: u16, kind: InvokeKind) -> Result<(), VmError> {
        //TODO add virtual method resolving
        let (class_name, method_name, descriptor) = self.class_and_method.get_constant_method_info_descriptor(index).expect("GIB MICH DIE METHODE");
        let args_count = MethodDescriptor::new(descriptor.clone()).args.len();
        let mut args = Vec::new();
        for _ in 0..args_count{
            args.push(self.stack.pop().unwrap());
        }
        let (method, object) = if kind == InvokeKind::STATIC{
            let class = vm.get_or_resolve_class(class_name.as_str())?;
            let method = class.find_method(method_name.as_str(), descriptor.as_str()).unwrap();
            let class_and_method = ClassAndMethod{class, method};
            let object = vm.get_static_class_object(class.id);
            (class_and_method, object)
        } else {
            let method = vm.resolve_class_method(class_name.as_str(), method_name.as_str(), descriptor.as_str())?;
            let object = if let Some(Value::Object(obj)) = self.stack.pop() {Some(obj)} else {None};
            (method, object)
        };

        println!("INVOKE{:?}: {}{} {:?}", kind, method_name, descriptor, object);
        let res = vm.invoke(method, object, args)?;
        if res.is_some(){
            self.stack.push(res.unwrap())
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
enum InvokeKind{
    STATIC,
    SPECIAL,
    VIRTUAL,
    DYNAMIC,
}