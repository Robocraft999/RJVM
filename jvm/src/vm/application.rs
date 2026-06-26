use std::cell::{Cell, RefCell, SyncUnsafeCell, UnsafeCell};
use std::fmt::Debug;
use crate::vm::class::ClassId;
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, Value};
use crate::vm::{jni, Context, VmError, VM};
use log::error;
use std::pin::Pin;
use crate::vm::class_path::ClassPath;
use crate::vm::constants::classes::{JAVA_LANG_CLASS, JAVA_LANG_INVOKE_METHOD_HANDLE, JAVA_LANG_INVOKE_METHOD_TYPE, JAVA_LANG_INVOKE_MHN, JAVA_LANG_REFLECT_METHOD, JAVA_LANG_STRING, JAVA_LANG_SYSTEM, JAVA_LANG_THREAD, JAVA_LANG_THREAD_GROUP, SUN_MISC_VM};
use crate::vm::constants::{THREAD_threadStatus_INDEX, THREAD_eetop_INDEX, THREAD_priority_INDEX};
use crate::vm::java_thread::{JavaThread, NORM_PRIORITY, RUNNABLE};

thread_local! {
    pub static JAVA_THREAD: Cell<*mut ()> = Cell::new(std::ptr::null_mut());
}

pub static THREADS: UnsafeCell<*mut ()> = UnsafeCell::new(std::ptr::null_mut());

pub fn thread<'a>() -> &'static mut JavaThread<'a>{
    unsafe {&mut* (JAVA_THREAD.get() as *mut JavaThread) }
}

pub struct Application<'a> {
    java_vm: Pin<Box<JavaVM<'a>>>,
    pub(crate) vm: Pin<Box<VM<'a>>>,
    main_thread: Pin<Box<JavaThread<'a>>>,
}

impl <'a> Application<'a> {
    pub fn new(class_path: ClassPath) -> Self {
        let mut main_thread = Box::pin(JavaThread::new(0));
        JAVA_THREAD.set(unsafe {main_thread.as_mut().get_unchecked_mut() } as *mut JavaThread as _);

        let vm = Box::pin(VM::new(class_path));

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

        Self { java_vm: javavm, vm, main_thread}
    }

    fn context(&self) -> Context<'a, '_> {
        let thread = unsafe {&mut* (JAVA_THREAD.get() as *mut JavaThread) };
        Context {thread, vm: &self.vm, java_vm: &self.java_vm}
    }

    fn init_system(&self) -> Result<(), VmError>{
        for (k,v) in self.vm.class_manager.class_loading_states.read()?.iter() {
            println!("Class: {:?}, state: {:?}", self.vm.find_class_by_id(ClassId(k.0)).unwrap().name, v);
        }
        let init = self.vm.resolve_class_method(JAVA_LANG_SYSTEM, "initializeSystemClass", "()V")?;
        JavaThread::invoke_subroutine(self.context(), init, None, vec![])?;
        Ok(())
    }

    fn handle_partial(&self, result: VMPartialResult<Option<Value<'a>>>) -> Option<Value<'a>> {
        match result {
            Ok(VMResultType::Successful(res)) => {
                println!("result: {res:?}");
                res
            }
            Ok(VMResultType::Interrupted(_, _)) => {
                self.handle_partial(JavaThread::invoke_frames_until(self.context(), -1))
            }
            Ok(VMResultType::ExceptionThrown) => {
                error!("Exception thrown");
                thread().debug_helper.print();
                self.vm.vm_debug_helper.print();
                panic!()
            }
            Err(error) => {
                error!("Error: {}", error);
                println!("Frames:");
                thread().call_stack.print_call_stack();
                thread().debug_helper.print();
                self.vm.vm_debug_helper.print();
                panic!()
            }
        }
    }

    pub fn run_and_catch_method(&self, class_name: &str, method_name: &str, method_descriptor: &str, args: Vec<Value<'a>>) {
        self.init_class(class_name);
        let main_method = self.vm.resolve_class_method(class_name, method_name, method_descriptor).unwrap();
        let _result = self.handle_partial(JavaThread::invoke_subroutine(self.context(), main_method, None, args));
    }

    fn init_class(&self, class_name: &str) {
        if let VMResultType::Interrupted(..) = self.context().get_or_initialize_class(class_name).unwrap().clone(){
            JavaThread::invoke_frames_until(self.context(), -1).unwrap();
        }
    }

    fn create_initial_thread_group(&self) -> Reference<'a> {
        let Ok(VMResultType::Successful(system_group)) = self.context().new_object(JAVA_LANG_THREAD_GROUP) else {
            thread().debug_helper.print();
            self.vm.vm_debug_helper.print();
            panic!("Could not allocate system thread group");
        };
        let system_init = self.vm.resolve_class_method(JAVA_LANG_THREAD_GROUP, "<init>", "()V").unwrap();
        let _ = self.handle_partial(JavaThread::invoke_subroutine(self.context(), system_init, Some(system_group), Vec::new()));

        let Ok(VMResultType::Successful(main_group)) = self.context().new_object(JAVA_LANG_THREAD_GROUP) else {
            thread().debug_helper.print();
            self.vm.vm_debug_helper.print();
            panic!("Could not allocate system thread group");
        };
        let name = self.vm.try_new_string_object("main").unwrap();
        let main_init = self.vm.resolve_class_method(JAVA_LANG_THREAD_GROUP, "<init>", "(Ljava/lang/ThreadGroup;Ljava/lang/String;)V").unwrap();
        let _ = self.handle_partial(JavaThread::invoke_subroutine(self.context(), main_init, Some(main_group), vec![Value::Reference(system_group), Value::Reference(name)]));

        main_group
    }
    fn create_initial_thread(&self, thread_group: Reference<'a>) -> Reference<'a>{
        let Ok(VMResultType::Successful(thread)) = self.context().new_object(JAVA_LANG_THREAD) else {
            thread().debug_helper.print();
            self.vm.vm_debug_helper.print();
            panic!("Could not allocate system thread group");
        };
        thread.set_field(THREAD_eetop_INDEX, Value::Long(crate::vm::application::thread().id as i64));
        thread.set_field(THREAD_priority_INDEX, Value::Integer(NORM_PRIORITY));
        crate::vm::application::thread().thread_obj_id.replace(thread.id);

        let name = self.vm.try_new_string_object("main").unwrap();
        let init = self.vm.resolve_class_method(JAVA_LANG_THREAD, "<init>", "(Ljava/lang/ThreadGroup;Ljava/lang/String;)V").unwrap();
        let _ = self.handle_partial(JavaThread::invoke_subroutine(self.context(), init, Some(thread), vec![Value::Reference(thread_group), Value::Reference(name)]));

        thread
    }

    pub fn startup(&mut self){
        self.init_class(JAVA_LANG_STRING);
        self.init_class(JAVA_LANG_SYSTEM);
        self.init_class(JAVA_LANG_THREAD_GROUP);
        let thread_group_ref = self.create_initial_thread_group();
        self.init_class(JAVA_LANG_THREAD);
        let thread_obj_ref = self.create_initial_thread(thread_group_ref);
        thread().thread_obj_id.replace(thread_obj_ref.id);
        thread_obj_ref.set_field(THREAD_threadStatus_INDEX, Value::Integer(RUNNABLE));

        self.init_class(JAVA_LANG_CLASS);
        self.init_class(JAVA_LANG_REFLECT_METHOD);
        // java/lang/ref/Finalizer

        match self.init_system() {
            Ok(_) => {}
            Err(error) => {
                println!("'{}'", error);
                error!("Init System: {}", error);
                thread().debug_helper.print();
                self.vm.vm_debug_helper.print();
                panic!();
            }
        }
        self.init_class(JAVA_LANG_INVOKE_METHOD_TYPE);
        self.init_class(JAVA_LANG_INVOKE_METHOD_HANDLE);
        self.init_class(JAVA_LANG_INVOKE_MHN);
        /*#[cfg(feature = "debug")]
        {
            self.vm.debug_helper.exception_helper.print();
        }*/
        println!("Init complete. Starting Main Program");
    }
}