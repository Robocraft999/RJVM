use std::pin::Pin;
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::result::VMResultType;
use crate::vm::value::Value;
use crate::vm::{jni, VmError, VM};
use log::error;

pub struct Application<'a>{
    java_vm: Pin<Box<JavaVM<'a>>>,
    pub(crate) vm: Pin<Box<VM<'a>>>,
}

impl <'a> Application<'a>{
    pub fn new(vm: Pin<Box<VM<'a>>>) -> Self{
        let env = JNIEnv{
            methods: jni::env_function_table::METHODS,
            vm: vm.as_ref().get_ref(),
            pvm: std::ptr::null(),
        };
        let mut javavm = Box::pin(JavaVM{
            methods: jni::vm_function_table::METHODS,
            env
        });
        let javavm_ptr: *const JavaVM<'a> = javavm.as_ref().get_ref();
        unsafe {
            //SAFETY: JavaVM is !UnPin so it will not be moved and get_unchecked_mut has to be used
            let javavm_mut = Pin::as_mut(&mut javavm).get_unchecked_mut();
            javavm_mut.env.pvm = javavm_ptr;
        }

        println!("javavm: {:p}", javavm);
        Self { java_vm: javavm, vm }
    }

    fn init_vm(&self) -> Result<(), VmError>{
        if let VMResultType::Interrupted(..) = self.vm.get_or_resolve_class("sun/misc/VM")?{
            self.vm.invoke_frames_until(&self.java_vm, -1)?;
        }

        Ok(())
    }

    fn init_system(&self) -> Result<(), VmError>{
        if let VMResultType::Interrupted(..) = self.vm.get_or_resolve_class("java/lang/System")?{
            self.vm.invoke_frames_until(&self.java_vm, -1)?;
        }
        let init = self.vm.try_resolve_class_method("java/lang/System", "initializeSystemClass", "()V")?;
        self.vm.invoke_new_frame(&self.java_vm, init, None, vec![])?;
        Ok(())
    }

    pub fn run_and_catch_method(&self, class_name: &str, method_name: &str, method_descriptor: &str, args: Vec<Value<'a>>){
        if let VMResultType::Interrupted(..) = self.vm.get_or_resolve_class(class_name).unwrap().clone(){
            self.vm.invoke_frames_until(&self.java_vm, -1).unwrap();
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
                #[cfg(feature = "debug")]
                {
                    self.vm.debug_helper.exception_helper.print();
                }
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