use crate::vm::jni::types::JavaVM;
use crate::vm::result::VMResultType;
use crate::vm::value::Value;
use crate::vm::{VmError, VM};
use log::error;

pub struct Application<'a>{
    java_vm: JavaVM<'a>,
    vm: VM<'a>,
}

impl <'a> Application<'a>{
    pub fn new(java_vm: JavaVM<'a>, vm: VM<'a>) -> Self{
        Self { java_vm, vm }
    }

    fn init_vm(&self) -> Result<(), VmError>{
        if let VMResultType::NeedsClassInit(classes, _) = self.vm.get_or_resolve_class("sun/misc/VM")?{
            for frame in classes{
                self.vm.invoke_current_frame(&self.java_vm)?;
            }
        }

        Ok(())
    }

    fn init_system(&self) -> Result<(), VmError>{
        if let VMResultType::NeedsClassInit(classes, _) = self.vm.get_or_resolve_class("java/lang/System")?{
            for frame in classes{
                self.vm.invoke_current_frame(&self.java_vm)?;
            }
        }
        let init = self.vm.try_resolve_class_method("java/lang/System", "initializeSystemClass", "()V")?;
        self.vm.invoke_new_frame(&self.java_vm, init, None, vec![])?;
        Ok(())
    }

    pub fn run_and_catch_method(&self, class_name: &str, method_name: &str, method_descriptor: &str, args: Vec<Value<'a>>){
        if let VMResultType::NeedsClassInit(classes, _) = self.vm.get_or_resolve_class(class_name).unwrap().clone(){
            for frame in classes{
                self.vm.invoke_current_frame(&self.java_vm).unwrap();
            }
        }
        let main_method = self.vm.try_resolve_class_method(class_name, method_name, method_descriptor).unwrap();
        let result = self.vm.invoke_new_frame(&self.java_vm, main_method, None, args);
        match result {
            Ok(res) => {
                println!("result: {res:?}");
            }
            Err(error) => {
                error!("Error: {}", error);
                println!("Frames:");
                self.vm.call_stack.print_call_stack();
            }
        }
    }
    
    pub fn startup(&self){
        match self.init_vm() {
            Ok(_) => {}
            Err(error) => {
                error!("Init VM: {}", error);
                panic!();
            }
        };
        match self.init_system(){
            Ok(_) => {}
            Err(error) => {
                println!("'{}'", error);
                error!("Init System: {}", error);
                #[cfg(feature = "debug")]
                {
                    self.vm.debug_helper.exception_helper.print();
                }
                panic!();
            }
        }
        #[cfg(feature = "debug")]
        {
            self.vm.debug_helper.exception_helper.print();
        }
        println!("Init complete. Starting Main Program");
    }
}