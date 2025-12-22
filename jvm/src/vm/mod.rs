use callstack::CallStack;
use cesu8::{from_java_cesu8, to_java_cesu8, Cesu8DecodingError};
use log::{debug, error, info, trace};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use std::str::Utf8Error;
use thiserror::Error;

use crate::access_flags::MethodFlag;
use crate::attribute::{BootstrapMethods, Code, ExceptionTable, ProgramCounter, VisibleRuntimeAnnotations};
use crate::bytecode::Instruction;
use crate::constants::ConstantPool;
use crate::error::ClassParseError;
use crate::field_info::{FieldType, PrimitiveType};
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::vm::bytecode::InstructionBlock;
use crate::vm::call_frame::CallFrame;
use crate::vm::class::{Class, ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::ResolvedClass;
use crate::vm::debug::DebugHelper;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::java_native_method_impl::{register_all_natives, NativeMethodRegistry};
use crate::vm::jni::types::JavaVM;
use crate::vm::r#unsafe::Unsafe;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType};
use crate::{get_or_init, get_or_init_special};
use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;

pub mod class_path;
pub mod class_path_entry;
pub mod class_manager;
mod java_error;
pub mod value;
mod call_frame;
mod callstack;
pub mod class;
mod gc;
mod java_native_method_impl;
mod r#unsafe;
pub mod result;
pub mod bytecode; //TODO move out from vm
mod executor;
mod debug;
pub mod jni;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub static_class_objects: RefCell<HashMap<ClassId, Reference<'a>>>,
    pub string_objects: RefCell<HashMap<String, Reference<'a>>>,
    pub class_objects: RefCell<HashMap<ClassId, Reference<'a>>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub currently_open_files: RefCell<HashMap<String, (Vec<u8>, usize)>>,
    pub current_thread: RefCell<Option<Reference<'a>>>,
    pub debug_helper: DebugHelper,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let mut class_manager = ClassManager::new(class_path);
        let mut native_method_registry = NativeMethodRegistry::new();
        register_all_natives(&mut native_method_registry);
        let unsafe_allocator = Unsafe::new();
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            unsafe_allocator,
            call_stack: CallStack::new(),
            static_class_objects: RefCell::new(HashMap::new()),
            string_objects: RefCell::new(HashMap::new()),
            class_objects: RefCell::new(HashMap::new()),
            native_method_registry,
            currently_open_files: RefCell::new(HashMap::new()),
            current_thread: RefCell::new(None),
            debug_helper: DebugHelper::new()
        }
    }

    pub fn dump_class_file(&mut self, class_name: &str) -> VMResult<()>{
        let class = self.get_or_resolve_class(class_name)?;
        info!("Class: {:?}", class);
        Ok(())
    }

    pub fn invoke_new_frame(&self, java_vm: &JavaVM, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
        self.call_stack.create_and_push_call_frame(class_and_method, object, args, false);
        self.invoke_current_frame(java_vm)
    }

    pub fn invoke_current_frame(&self, java_vm: &JavaVM) -> VMPartialResult<'a, Option<Value<'a>>> {
        let mut last_result: Option<VMResultType<Option<Value>>> = None;
        loop {
            let frame_amount = self.call_stack.len();
            let frame_index = if frame_amount > 0 {frame_amount - 1} else {frame_amount};
            //let frame_index = self.call_stack.frames.len() - 1;
            let current_result = if let Some(VMResultType::ExceptionThrown(error, ref throwable)) = last_result{
                VMResultType::ExceptionThrown(error, throwable.clone())
            } else {
                let class_and_method = self.call_stack.get_class_and_method_cloned();
                if class_and_method.method.is_native(){
                    self.execute_native(java_vm, class_and_method)?
                } else {
                    executor::execute(self)?
                }
            };
            last_result = None;
            match current_result{
                VMResultType::Ok(result) => {
                    let frame = self.call_stack.pop_call_frame();
                    if self.call_stack.is_empty(){
                        return Ok(VMResultType::Ok(result));
                    }
                    if let Some(value) = result{
                        if frame.should_push_return{
                            self.call_stack.push_operand_value(value);
                        }
                    }
                }
                VMResultType::NativeOk(result) => {
                    if self.call_stack.is_empty(){
                        return Ok(VMResultType::Ok(result));
                    }
                    let frame = self.call_stack.pop_call_frame_at(frame_index);
                    if let Some(value) = result{
                        //self.call_stack.push_operand_value(value);
                        if frame.should_push_return {
                            self.call_stack.operand_stacks.borrow_mut()[frame_index - 1].push(value);
                        }
                    }
                }
                VMResultType::NativeException(error, throwable) => {
                    if let VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message, origin)) = error {
                        let _frame = self.call_stack.pop_call_frame_at(frame_index);
                        let calling_frame_index = frame_index - 1;
                        let class_and_method = self.call_stack.frames.borrow().get(calling_frame_index).unwrap().class_and_method.clone();

                        let exception_table = class_and_method.method.get_exception_handlers();
                        let current_pc = &self.call_stack.pcs.borrow()[calling_frame_index];
                        //[unchecked] class already loaded by native
                        if let Some(handler_pc) = self.unchecked_resolve_exception_handler(exception_table, current_pc, thrown_class_name.as_str())? {
                            self.call_stack.pcs.borrow_mut()[calling_frame_index] = handler_pc;
                            self.call_stack.operand_stacks.borrow_mut()[calling_frame_index].push(throwable);
                            #[cfg(feature = "debug")]
                            {
                                self.debug_helper.exception_helper.push(format!("Handled Native {} by {}\n└-- thrown by {} with message: {}", thrown_class_name, class_and_method.format(), origin, message));
                            }
                        } else {
                            //unimplemented!("that's a problem, because we still have to run the initializer of the exception before searching for an exception handler in the stack")
                            self.call_stack.pop_call_frame();
                            last_result = Some(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message, origin)), throwable));
                            debug!("Exception handler not in this function {}", class_and_method.format());
                        }
                    } else {
                        unreachable!("Should not happen if all natives are configured correct")
                    }
                }
                VMResultType::CallPaused(new_frame) => {
                    /*self.call_stack.push_call_frame(new_frame)*/
                    },
                VMResultType::NeedsClassInit(classes, _) => {
                    /*for new_frame in classes {
                        self.call_stack.push_call_frame(new_frame);
                    }*/
                    let last_frame_index = self.call_stack.pcs.borrow().len() - classes.len() - 1;
                    let current_pc = self.call_stack.pcs.borrow()[last_frame_index];
                    let previous_pc = self.call_stack.frames.borrow()[last_frame_index].class_and_method.method.previous_pc(current_pc);
                    *self.call_stack.pcs.borrow_mut().get_mut(last_frame_index).unwrap() = ProgramCounter(previous_pc)
                }
                VMResultType::ExceptionThrown(error, throwable) => {
                    if let VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message, origin)) = error {
                        let last_frame_method_option = self.call_stack.frames.borrow().last().map(|e| e.class_and_method.clone());
                        if let Some(class_and_method) = last_frame_method_option {
                            let exception_table = class_and_method.method.get_exception_handlers();
                            let current_pc = &self.call_stack.get_pc();
                            //[unchecked] class already loaded by method
                            if let Some(handler_pc) = self.unchecked_resolve_exception_handler(exception_table, current_pc, thrown_class_name.as_str())?{
                                self.call_stack.set_pc(handler_pc.0);
                                self.call_stack.push_operand_value(throwable.clone());
                                #[cfg(feature = "debug")]
                                {
                                    self.debug_helper.exception_helper.push(format!("Handled {} by {}\n└-- thrown by {} with message: {}", thrown_class_name, class_and_method.format(), origin, message));
                                }
                                debug!("Exception thrown handled by {}", class_and_method.format());
                            } else {
                                self.call_stack.pop_call_frame();
                                last_result = Some(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message, origin)), throwable));
                                debug!("Exception handler not in this function {}", class_and_method.format());
                            }
                        } else {
                            #[cfg(feature = "debug")]
                            {
                                self.debug_helper.exception_helper.push(format!("Could not handle {} thrown by function {} with message: {}", thrown_class_name, origin, message));
                                self.debug_helper.exception_helper.print();
                            }
                            panic!("Could not handle {} thrown by function {} with message: {}", thrown_class_name, origin, message)
                        }
                    } else {
                        unreachable!("Could not handle {} ({:?}) thrown by a function", error, throwable);
                    }
                }
            }
            self.call_stack.print_call_stack();
        }
    }

    /*pub fn invoke_new_frame_old(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
        let frame = CallStack::create_call_frame(class_and_method, object, args);
        self.invoke_frame_old(frame)
    }*/
    /*
    main_frame
        func1
            func2
            func3
                try
                    func4
                catch
        func5

    push main_frame
    last_result = exe main_frame = CallPaused(func1)
            push func1
    cs = [main_frame, func1]

    last_result = exe func1 = CallPaused(func2)
            push func2
    cs = [main_frame, func1, func2]

    last_result = exe func2 = Ok(f2res)
            add f2res top (func1)
    cs = [main_frame, func1]

    last_result = exe func1 = CallPaused(func3)
            push func3
    cs = [main_frame, func1, func3]

    last_result = exe func3 = CallPaused(func4)
            push func4
    cs = [main_frame, func1, func3, func4]

    last_result = exe func4 = ExceptionThrown
            try_resolve_handler
                    found
                            last_frame.pc = handler_pc //func4
                    not found
                            pop func4
                            continue (cs=[main_frame, func1, func3])
    cs = [main_frame, func1, func3, func4]

    last_result = exe func4 = Ok(f4res)
            add f4res top (func3)
    cs = [main_frame, func1, func3]

    last_result = exe func3 = Ok(f2res)
            add f3res top (func1)
    cs = [main_frame, func1]

    last_result = exe func1 = Ok(f1res)
            add f1res top (main_frame)
    cs = [main_frame]

    last_result = exe main_frame = CallPaused(func5)
            push func5
    cs = [main_frame, func5]

    last_result = exe func5 = Ok(f5res)
            add f5res top (main_frame)
    cs = [main_frame]

    last_result = exe main_frame = Ok(main_res)
        return main_res
     */

    /*pub fn invoke_frame(&mut self, main_frame: CallFrame<'a>) -> VMPartialResult<'a, Option<Value<'a>>> {
        self.call_stack.push_call_frame(main_frame);
        let vm_ptr: *mut VM = self;
        let mut last_result: Option<VMResultType<Option<Value>>> = None;
        loop{
            let current_result = if let Some(VMResultType::ExceptionThrown(error, ref throwable)) = last_result{
                VMResultType::ExceptionThrown(error, throwable.clone())
            } else {
                let class_and_method = self.call_stack.frames.last().unwrap().class_and_method.clone();
                if class_and_method.method.is_native(){
                    self.execute_native(class_and_method)?
                } else {
                    self.call_stack.execute_top(vm_ptr)?
                }
            };
            last_result = None;
            match current_result {
                VMResultType::Ok(result) => {
                    if self.call_stack.frames.is_empty(){
                        return Ok(VMResultType::Ok(result));
                    }
                    self.call_stack.add_to_top_stack(result);
                }
                VMResultType::CallPaused(new_frame) => self.call_stack.push_call_frame(new_frame),
                VMResultType::NeedsClassInit(classes, _) => {
                    for new_frame in classes {
                        self.call_stack.push_call_frame(new_frame);
                    }
                }
                VMResultType::ExceptionThrown(error, throwable) => {
                    if let VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)) = error {
                        if let Some(error_frame) = self.call_stack.frames.last(){
                            let class_and_method = error_frame.class_and_method.clone();
                            let exception_table = class_and_method.method.get_exception_handlers().clone();
                            let current_pc = error_frame.pc.clone();
                            if let Some(handler_pc) = get_or_init!(self.try_resolve_exception_handler(exception_table, current_pc, thrown_class_name.as_str())?){
                                let error_frame = self.call_stack.frames.last_mut().unwrap();
                                error_frame.pc = handler_pc;
                                error_frame.stack.push(throwable.clone());
                                debug!("Exception thrown handled by {}", class_and_method.format());
                            } else {
                                self.call_stack.pop_call_frame();
                                last_result = Some(VMResultType::ExceptionThrown(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message)), throwable));
                                debug!("Exception handler not in this function {}", class_and_method.format());
                            }
                        } else {
                            panic!("Could not handle {} thrown by a function with message: {}", thrown_class_name, message)
                        }
                    } else {
                        unreachable!("Could not handle {} thrown by a function", error);
                    }
                }
            }
            self.call_stack.print_call_stack();
        }
    }*/

    fn execute_native(&self, java_vm: &JavaVM, class_and_method: ClassAndMethod<'a>) -> VMPartialResult<'a, Option<Value<'a>>> {
        //let call_frame = self.call_stack.pop_call_frame();
        
        let object = if class_and_method.method.is_static() {
            None
        } else {
            match self.call_stack.load_local(0) {
                Some(local) => {
                    Some(local.expect_reference()?)
                },
                None => None
            }
        };
        let args = self.call_stack.locals_stack.borrow().last().unwrap()
            .iter()
            .cloned()
            .skip(if object.is_none() {0} else {1})
            .take_while(|value| value != &Value::Uninitialized)
            .collect::<Vec<_>>();
        let try_native = NativeMethodRegistry::invoke(self, java_vm, &class_and_method, object, args);
        debug!("TTT {:?}", try_native);
        if let Some(native) = try_native {
            native
        } else {
            debug!("native not found");
            if class_and_method.method.descriptor.return_type.is_some(){
                Err(VmError::MethodCallError(format!("native {} returns a value which is probably used", class_and_method.format())))
            } else {
                Ok(VMResultType::Ok(None))
            }
        }
    }

    fn resolve_exception_handler(&self, exception_table: &ExceptionTable, pc: &ProgramCounter, thrown_class_name: &str) -> VMPartialResult<'a, Option<ProgramCounter>>{
        let _ = self.get_or_resolve_class(thrown_class_name)?;
        self.unchecked_resolve_exception_handler(exception_table, pc, thrown_class_name).map(VMResultType::Ok)
    }

    fn unchecked_resolve_exception_handler(&self, exception_table: &ExceptionTable, pc: &ProgramCounter, thrown_class_name: &str) -> VMResult<Option<ProgramCounter>>{
        for handler in &exception_table.0 {
            let can_handle = match handler.catch_type {
                Some(ref class_name) => {
                    self.unchecked_check_if_subclass_of(class_name.as_str(), thrown_class_name)?
                }
                None => true
            };
            if can_handle{
                //FIXME check if end_pc is inclusive or exclusive
                if handler.start_pc.0 <= pc.0 && pc.0 <= handler.end_pc.0{
                    return Ok(Some(handler.handler_pc));
                }
            }
        }
        Ok(None)
    }

    pub fn get_or_resolve_class(&self, class_name: &str) -> VMPartialResult<'a, ClassRef<'a>>{
        let resolved = self.class_manager.get_or_resolve_class(class_name)?;
        if let ResolvedClass::NewClass(to_init) = &resolved{
            Ok(VMResultType::NeedsClassInit(
                to_init.to_initialize
                    .iter()
                    .map(|class| self.init_class(class))
                    .filter(Option::is_some)
                    .map(Option::unwrap)
                    .collect(),
                true
            ))
            /*for class in to_init.to_initialize.iter(){
                if let Some(clinit) = self.init_class(class)?{
                    //self.invoke_frame_on_stack(clinit)?;
                }
            }*/
        } else {
            Ok(VMResultType::Ok(resolved.get_class()))
        }
    }

    pub fn try_get_class(&self, class_name: &str) -> VMResult<ClassRef<'a>>{
        self.find_class_by_name(class_name.to_owned()).ok_or(VmError::ClassNotLoadedError(format!("[try_get_class]: Class not loaded: {}", class_name)))
    }

    pub fn define_class(&self, class_name: &str, bytes: Vec<u8>) -> VMPartialResult<'a, Reference<'a>>{
        let resolved = self.find_class_by_name(class_name.to_string());
        if resolved.is_none() {
            let to_init = self.class_manager.parse_and_load_class(class_name, class_name, None, bytes)?;
            return Ok(VMResultType::NeedsClassInit(
                to_init.to_initialize
                    .iter()
                    .map(|class| self.init_class(class))
                    .filter(Option::is_some)
                    .map(Option::unwrap)
                    .collect(),
                true
            ))
        }
        let resolved = resolved.unwrap();
        self.new_class_object_by_class(resolved)
    }

    fn init_class(&self, class: ClassRef<'a>) -> Option<()>{
        if class.transitive_field_count > 0 && !class.is_array(){
            let static_object = self.new_object_from_class(class);
            self.static_class_objects.borrow_mut().insert(class.id, static_object);
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                self.call_stack.create_and_push_call_frame(class_and_method, Some(static_object), Vec::new(), false);
                return Some(());
            }
        }

        None
    }

    pub fn resolve_class_method(&self, class_name: &str, method_name: &str, descriptor: &str) -> VMPartialResult<'a, ClassAndMethod<'a>>{
        let result = self.get_or_resolve_class(class_name);
        result.and_then(|class| {
            let class = get_or_init!(class);
            self.get_class_method(class, method_name, descriptor).map(|e| VMResultType::Ok(e))
        })
    }
    
    pub fn get_class_method(&self, class: ClassRef<'a>, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        class
            .find_method(method_name, descriptor)
            .map(|method| ClassAndMethod{ class, method})
            .ok_or(VmError::JavaException(JavaError::MethodNotFoundException(method_name.to_string())))
    }

    pub fn try_resolve_class_method(&self, class_name: &str, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        if let VMResultType::Ok(method) = self.resolve_class_method(class_name, method_name, descriptor)? {
            Ok(method)
        } else {
            Err(VmError::ClassNotLoadedError(format!("[try_resolve_class_method]: Class not loaded: {}", class_name)))
        }
    }

    pub fn new_object(&self, class_name: &str) -> VMPartialResult<'a, Reference<'a>>{
        get_or_init_special!(self.get_or_resolve_class(class_name)?, |class| Ok(VMResultType::Ok(self.new_object_from_class(class))))
    }

    pub fn try_new_object(&self, class_name: &str) -> VMResult<Reference<'a>>{
        let result = self.new_object(class_name)?;
        if let VMResultType::Ok(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError(format!("[try_new_object]: Class not loaded: {}", class_name)))
        }
    }

    pub fn new_object_from_class(&self, class: ClassRef<'a>) -> Reference<'a>{
        info!("CC[{:?}] = {}", class.id, class.name);
        self.object_allocator.allocate_object(class)
    }

    pub fn get_static_class_object(&self, id: ClassId) -> Option<Reference<'a>>{
        self.static_class_objects.borrow().get(&id).cloned()
    }

    pub fn new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMPartialResult<'a, Reference<'a>>{
        let (class_name, component_type) = if let FieldType::Array(class_name, component_type) = array_field_type {
            (class_name, component_type)
        } else {
            unreachable!("The field type for creating an array has to be an array field type")
        };
        get_or_init_special!(self.get_or_resolve_class(class_name.as_str())?, |class| Ok(VMResultType::Ok(self.object_allocator.allocate_array(class, dims, *component_type, content))))
    }

    pub fn try_new_array(&self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMResult<Reference<'a>>{
        let result = self.new_array(dims, array_field_type, content)?;
        if let VMResultType::Ok(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_object]: Class not loaded".to_string()))
        }
    }

    pub fn try_new_string_object(&self, string: String) -> VMResult<Reference<'a>>{
        let result = self.new_string_object(string)?;
        if let VMResultType::Ok(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_string_object]: Class not loaded".to_string()))
        }
    }
    
    pub fn new_string_object(&self, string: String) -> VMPartialResult<'a, Reference<'a>>{
        if self.string_objects.borrow().contains_key(&string){
            return Ok(VMResultType::Ok(self.string_objects.borrow()[&string]))
        }
        
        let char_array: Vec<Value<'a>> = string.chars().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);

        let char_array = Value::Reference(get_or_init!(self.new_array(1, FieldType::Primitive(PrimitiveType::Char).to_array_field_type(1), char_array)?));

        let string_object = get_or_init!(self.new_object("java/lang/String")?);

        string_object.set_field(0, char_array);
        string_object.set_field(1, Value::Integer(0));
        string_object.set_field(6, Value::Integer(0));

        self.string_objects.borrow_mut().insert(string, string_object);
        Ok(VMResultType::Ok(string_object))
    }

    pub fn extract_string_from_object(value: &Value<'a>) -> VMResult<String>{
        if let Value::Reference(reference) = value{
            let chars = reference.get_field(0);
            return Self::extract_string_from_char_arr(&chars);
        }
        Err(VmError::ValidationError(format!( "Expected String Object but found: {:?}", value)))
    }
    
    pub fn extract_string_from_char_arr(chars: &Value<'a>) -> VMResult<String>{
        if let Value::Reference(char_ref) = chars {
            if let ReferenceType::Array(_, _, content) = &char_ref.reference_type{
                let chars: Vec<u8> = content.borrow().iter().map(|v| if let Value::Integer(val) = v {*val as u8} else {0}).collect();
                let string = from_java_cesu8(chars.as_slice())?.to_string();
                return Ok(string);
            }
        }
        Err(VmError::ValidationError(format!( "Expected CharArray but found: {:?}", chars)))
    }

    pub fn new_class_object(&self, class_name: String, class_id: ClassId) -> VMPartialResult<'a, Reference<'a>>{
        if !self.class_objects.borrow().contains_key(&class_id){
            let class_object = get_or_init!(self.new_object("java/lang/Class")?);
            let string_object = get_or_init!(self.new_string_object(class_name.replace("/", "."))?);

            //name
            class_object.set_field(5, Value::Reference(string_object));

            self.class_objects.borrow_mut().insert(class_id, class_object);
            Ok(VMResultType::Ok(class_object))
        } else {
            Ok(VMResultType::Ok(self.class_objects.borrow()[&class_id]))
        }
    }

    pub fn new_class_object_by_class(&self, class: ClassRef<'a>) -> VMPartialResult<'a, Reference<'a>>{
        let class_id = class.id;
        let class_name = class.name.clone();
        self.new_class_object(class_name, class_id)
    }

    pub fn new_class_object_by_name(&self, class_name: String) -> VMPartialResult<'a, Reference<'a>> {
        let class_id = get_or_init_special!(self.get_or_resolve_class(class_name.as_str())?, |v: ClassRef| v.id);
        self.new_class_object(class_name, class_id)
    }

    pub fn extract_class_from_class_object(&self, object: Reference<'a>) -> VMPartialResult<'a, ClassRef<'a>>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        let class = get_or_init!(self.get_or_resolve_class(name.as_str())?);

        Ok(VMResultType::Ok(class))
    }
    
    pub fn extract_class_name_from_class_object(object: Reference<'a>) -> VMResult<String>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        Ok(name)
    }

    pub fn unchecked_check_if_subclass_of(&self, class_name: &str, of_name: &str) -> VMResult<bool>{
        let mut current_class = self.try_get_class(of_name)?;
        loop {
            if current_class.name == class_name {
                return Ok(true);
            }
            if let Some(super_class) = current_class.superclass {
                current_class = super_class;
            } else {
                return Ok(false);
            }
        }
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_id(class_id)
    }

    pub fn find_class_by_name(&self, name: String) -> Option<ClassRef<'a>>{
        self.class_manager.find_class_by_name(name.as_str())
    }
}

impl !Unpin for VM<'_>{}

impl Drop for VM<'_>{
    fn drop(&mut self) {
        error!("VM drop: {:p}", self);
    }
}

#[derive(Error, Debug, Clone)]
pub enum VmError{
    #[error("{0}")]
    JavaException(#[from] JavaError),

    #[error("")]
    ParseError(#[from] ClassParseError),

    #[error("Methodcall to {0} failed")]
    MethodCallError(String),

    #[error("Validation failed: expected: {0}")]
    ValidationError(String),

    #[error("{0}")]
    CESU8Error(#[from] Cesu8DecodingError),

    #[error("{0}")]
    ClassNotLoadedError(String),

    #[error("{0}")]
    Unspecified(String),
}