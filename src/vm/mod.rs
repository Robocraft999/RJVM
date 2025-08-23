use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::rc::Rc;
use std::str::Utf8Error;
use cesu8::{from_java_cesu8, to_java_cesu8, Cesu8DecodingError};
use callstack::CallStack;
use log::{debug, error, info, trace};
use thiserror::Error;

use class_manager::ClassManager;
use class_path::ClassPath;
use value::Value;
use crate::access_flags::MethodFlag;
use crate::attribute::{BootstrapMethods, Code, ExceptionTable, ProgramCounter, VisibleRuntimeAnnotations};
use crate::error::ClassParseError;
use crate::field_info::{FieldType, PrimitiveType};
use crate::{get_or_init, get_or_init_special};
use crate::constants::ConstantPool;
use crate::method_info::{MethodDescriptor, MethodInfo};
use crate::vm::call_frame::CallFrame;
use crate::vm::class::{Class, ClassAndMethod, ClassId, ClassRef};
use crate::vm::class_manager::ResolvedClass;
use crate::vm::gc::ObjectAllocator;
use crate::vm::java_error::JavaError;
use crate::vm::java_native_method_impl::{NativeMethodRegistry, register_all_natives};
use crate::vm::r#unsafe::Unsafe;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType};

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
mod bytecode;

pub struct VM<'a>{
    pub class_manager: ClassManager<'a>,
    pub call_stack: CallStack<'a>,
    pub object_allocator: ObjectAllocator<'a>,
    pub unsafe_allocator: Unsafe,
    pub static_class_objects: HashMap<ClassId, Reference<'a>>,
    pub string_objects: HashMap<String, Reference<'a>>,
    pub class_objects: HashMap<ClassId, Reference<'a>>,
    pub native_method_registry: NativeMethodRegistry<'a>,
    pub currently_open_files: HashMap<String, (Vec<u8>, usize)>,
    pub current_thread: Option<Reference<'a>>,
    pub init_call_stack: CallStack<'a>,
}

impl<'a> VM<'a>{
    pub fn new(class_path: ClassPath) -> Self{
        let mut class_manager = ClassManager::new(class_path);
        let mut native_method_registry = NativeMethodRegistry::new();
        register_all_natives(&mut native_method_registry);
        let unsafe_allocator = Unsafe::new();
        let dummy_class = class_manager.classes.alloc(Class {
            id: ClassId(0),
            name: "internal/Returner".to_string(),
            source_file: None,
            constants: ConstantPool(vec![]),
            flags: vec![],
            superclass: None,
            interfaces: vec![],
            fields: vec![],
            methods: vec![MethodInfo {
                flags: vec![],
                name: "dummyReturn".to_string(),
                descriptor: MethodDescriptor::new("()Ljava/lang/Object;".to_string()),
                deprecated: false,
                code: Some(Code {
                    max_stack: 1,
                    max_locals: 1,
                    code: vec![0xb0], //ARETURN
                    attributes: vec![],
                    line_number_table: None,
                    exception_table: ExceptionTable(vec![]),
                }),
                attributes: vec![],
                exceptions: None,
            },
            MethodInfo{
                flags: vec![],
                name: "dummyThrow".to_string(),
                descriptor: MethodDescriptor::new("()V".to_string()),
                deprecated: false,
                code: Some(Code{
                    max_stack: 1,
                    max_locals: 1,
                    code: vec![0xbf], //ATHROW
                    attributes: vec![],
                    line_number_table: None,
                    exception_table: ExceptionTable(vec![]),
                }),
                attributes: vec![],
                exceptions: None,
            }],
            annotations: VisibleRuntimeAnnotations(vec![]),
            bootstrap_methods: BootstrapMethods(Vec::new()),
            transitive_field_count: 0,
            first_field_index: 0,
            array_info: None,
        });
        let class_ref = unsafe {
            let class_ptr: *const Class<'a> = dummy_class;
            &*class_ptr
        };
        class_manager.classes_by_id.insert(class_ref.id, class_ref);
        Self{
            class_manager,
            object_allocator: ObjectAllocator::new(),
            unsafe_allocator,
            call_stack: CallStack::new(),
            static_class_objects: HashMap::new(),
            string_objects: HashMap::new(),
            class_objects: HashMap::new(),
            native_method_registry,
            currently_open_files: HashMap::new(),
            current_thread: None,
            init_call_stack: CallStack::new(),
        }
    }

    pub fn dump_class_file(&mut self, class_name: &str) -> VMResult<()>{
        let class = self.get_or_resolve_class(class_name)?;
        info!("Class: {:?}", class);
        Ok(())
    }

    pub fn invoke_new_frame(&mut self, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value<'a>>) -> VMPartialResult<'a, Option<Value<'a>>>{
        let frame = CallStack::create_call_frame(class_and_method, object, args);
        self.invoke_frame(frame)
    }
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

    pub fn invoke_frame(&mut self, main_frame: CallFrame<'a>) -> VMPartialResult<'a, Option<Value<'a>>> {
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
    }

    fn execute_native(&mut self, class_and_method: ClassAndMethod<'a>) -> VMPartialResult<'a, Option<Value<'a>>> {
        let call_frame = self.call_stack.pop_call_frame();
        let object = if class_and_method.method.is_static() {
            None
        } else {
            match call_frame.locals.get(0) {
                Some(local) => {
                    Some(local.expect_reference()?)
                },
                None => None
            }
        };
        let args = call_frame.locals
            .iter()
            .cloned()
            .skip(if object.is_none() {0} else {1})
            .take_while(|value| value != &Value::Uninitialized)
            .collect::<Vec<_>>();
        let try_native = NativeMethodRegistry::invoke(self, &class_and_method, object, args);
        if let Some(native) = try_native {
            Ok(match native? {
                VMResultType::Ok(value) => {VMResultType::Ok(value)}
                VMResultType::CallPaused(frame) => {
                    self.call_stack.push_call_frame(call_frame);
                    VMResultType::CallPaused(frame)
                }
                VMResultType::ExceptionThrown(error, throwable) => {
                    self.call_stack.push_call_frame(call_frame);
                    VMResultType::ExceptionThrown(error, throwable)
                }
                VMResultType::NeedsClassInit(classes, reenter) => {
                    if reenter {
                        self.call_stack.push_call_frame(call_frame);
                    }
                    VMResultType::NeedsClassInit(classes, reenter)
                }
            })
        } else {
            if class_and_method.method.descriptor.return_type.is_some(){
                Err(VmError::MethodCallError(format!("native {} returns a value which is probably used", class_and_method.format())))
            } else {
                Ok(VMResultType::Ok(None))
            }
        }
    }

    fn try_resolve_exception_handler(&mut self, exception_table: ExceptionTable, pc: ProgramCounter, thrown_class_name: &str) -> VMPartialResult<'a, Option<ProgramCounter>>{
        for handler in exception_table.0 {
            let can_handle = match handler.catch_type {
                Some(ref class_name) => {
                    get_or_init!(self.check_if_subclass_of(class_name.as_str(), thrown_class_name)?)
                }
                None => true
            };
            if can_handle{
                //FIXME check if end_pc is inclusive or exclusive
                if handler.start_pc.0 <= pc.0 && pc.0 <= handler.end_pc.0{
                    return Ok(VMResultType::Ok(Some(handler.handler_pc)));
                }
            }
        }
        Ok(VMResultType::Ok(None))
    }

    pub fn get_or_resolve_class(&mut self, class_name: &str) -> VMPartialResult<'a, ClassRef<'a>>{
        let resolved = self.class_manager.get_or_resolve_class(class_name)?;
        //FIXME maybe make this global
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

    pub fn define_class(&mut self, class_name: &str, bytes: Vec<u8>) -> VMPartialResult<'a, Reference<'a>>{
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

    fn init_class(&mut self, class: ClassRef<'a>) -> Option<CallFrame<'a>>{
        if class.transitive_field_count > 0 && !class.is_array(){
            let static_object = self.new_object_from_class(class);
            self.static_class_objects.insert(class.id, static_object);
            if let Some(clinit_method) = class.find_method("<clinit>", "()V"){
                let class_and_method = ClassAndMethod{
                    class,
                    method: clinit_method,
                };
                return Some(CallStack::create_call_frame(class_and_method, Some(static_object), Vec::new()));
            }
        }

        None
    }

    pub fn resolve_class_method(&mut self, class_name: &str, method_name: &str, descriptor: &str) -> VMPartialResult<'a, ClassAndMethod<'a>>{
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

    pub fn try_resolve_class_method(&mut self, class_name: &str, method_name: &str, descriptor: &str) -> VMResult<ClassAndMethod<'a>>{
        if let VMResultType::Ok(method) = self.resolve_class_method(class_name, method_name, descriptor)? {
            Ok(method)
        } else {
            Err(VmError::ClassNotLoadedError(format!("[try_resolve_class_method]: Class not loaded: {}", class_name)))
        }
    }

    pub fn new_object(&mut self, class_name: &str) -> VMPartialResult<'a, Reference<'a>>{
        get_or_init_special!(self.get_or_resolve_class(class_name)?, |class| Ok(VMResultType::Ok(self.new_object_from_class(class))))
    }

    pub fn try_new_object(&mut self, class_name: &str) -> VMResult<Reference<'a>>{
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
        self.static_class_objects.get(&id).cloned()
    }

    pub fn new_array(&mut self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMPartialResult<'a, Reference<'a>>{
        let (class_name, component_type) = if let FieldType::Array(class_name, component_type) = array_field_type {
            (class_name, component_type)
        } else {
            unreachable!("The field type for creating an array has to be an array field type")
        };
        get_or_init_special!(self.get_or_resolve_class(class_name.as_str())?, |class| Ok(VMResultType::Ok(self.object_allocator.allocate_array(class, dims, *component_type, content))))
    }

    pub fn try_new_array(&mut self, dims: usize, array_field_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> VMResult<Reference<'a>>{
        let result = self.new_array(dims, array_field_type, content)?;
        if let VMResultType::Ok(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_object]: Class not loaded".to_string()))
        }
    }

    pub fn try_new_string_object(&mut self, string: String) -> VMResult<Reference<'a>>{
        let result = self.new_string_object(string)?;
        if let VMResultType::Ok(object) = result {
            Ok(object)
        } else {
            Err(VmError::ClassNotLoadedError("[try_new_string_object]: Class not loaded".to_string()))
        }
    }
    
    pub fn new_string_object(&mut self, string: String) -> VMPartialResult<'a, Reference<'a>>{
        if self.string_objects.contains_key(&string){
            return Ok(VMResultType::Ok(self.string_objects[&string]))
        }
        
        let char_array: Vec<Value<'a>> = string.chars().map(|c| Value::Integer(c as i32)).collect();
        let char_array = RefCell::new(char_array);

        let char_array = Value::Reference(get_or_init!(self.new_array(1, FieldType::Primitive(PrimitiveType::Char).to_array_field_type(1), char_array)?));

        let string_object = get_or_init!(self.new_object("java/lang/String")?);

        string_object.set_field(0, char_array);
        string_object.set_field(1, Value::Integer(0));
        string_object.set_field(6, Value::Integer(0));

        self.string_objects.insert(string, string_object);
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

    pub fn new_class_object(&mut self, class_name: String, class_id: ClassId) -> VMPartialResult<'a, Reference<'a>>{
        if !self.class_objects.contains_key(&class_id){
            let class_object = get_or_init!(self.new_object("java/lang/Class")?);
            let string_object = get_or_init!(self.new_string_object(class_name)?);

            class_object.set_field(5, Value::Reference(string_object));

            self.class_objects.insert(class_id, class_object);
            Ok(VMResultType::Ok(class_object))
        } else {
            Ok(VMResultType::Ok(self.class_objects[&class_id]))
        }
    }

    pub fn new_class_object_by_class(&mut self, class: ClassRef<'a>) -> VMPartialResult<'a, Reference<'a>>{
        let class_id = class.id;
        let class_name = class.name.clone();
        self.new_class_object(class_name, class_id)
    }

    pub fn new_class_object_by_name(&mut self, class_name: String) -> VMPartialResult<'a, Reference<'a>> {
        let class_id = get_or_init_special!(self.get_or_resolve_class(class_name.as_str())?, |v: ClassRef| v.id);
        self.new_class_object(class_name, class_id)
    }

    pub fn extract_class_from_class_object(&mut self, object: Reference<'a>) -> VMPartialResult<'a, ClassRef<'a>>{
        let name_object = object.get_field(5);
        let name = VM::extract_string_from_object(&name_object)?;
        let name = name.replace(".", "/");
        let class = get_or_init!(self.get_or_resolve_class(name.as_str())?);

        Ok(VMResultType::Ok(class))
    }
    
    pub fn check_if_subclass_of(&mut self, class_name: &str, of_name: &str) -> VMPartialResult<'a, bool>{
        //println!("HELPME  {}", class_name);
        let mut current_class = get_or_init!(self.get_or_resolve_class(of_name)?);
        loop {
            //println!("HELPME2 {}", current_class.name);
            if current_class.name == class_name {
                return Ok(VMResultType::Ok(true));
            }
            if let Some(super_class) = current_class.superclass {
                current_class = super_class;
            } else {
                return Ok(VMResultType::Ok(false));
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
}