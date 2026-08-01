use crate::vm::callstack::CallStack;
use crate::vm::class::{ClassAndMethod, ClassAndMethodId, ClassRef};
use crate::vm::constants::THROWABLE_detailsMessage_INDEX;
use crate::vm::debug::DebugHelper;
use crate::vm::java_error::JavaError;
use crate::vm::jni::types::{JNIEnv, JavaVM};
use crate::vm::monitoring::MonitorAssociate;
use crate::vm::native::NativeMethodRegistry;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{RefId, Reference, Value};
use crate::vm::{executor, Context, ProgramCounter, VmError};
use log::{debug, warn};
use parking_lot::{Mutex, RwLock};
use std::cell::RefCell;
use std::pin::Pin;
use std::sync::Arc;
use std::thread::Thread;

pub type TID = u32;
pub const NORM_PRIORITY: i32 = 5;
pub const RUNNABLE: i32 = 1 + 4; //jvmti: alive + runnable

#[derive(Debug, PartialEq)]
pub enum ThreadState {
    Running,
    Sleeping,
    Waiting(MonitorAssociate),
    Blocked,
    Parked,
}

#[derive(Debug)]
pub struct ThreadMeta {
    pub id: TID,
    pub os_thread: Thread,

    pub interrupted: RwLock<bool>,
    pub state: RwLock<ThreadState>,
    pub unsafe_unpark_count: Mutex<usize>,
}

impl ThreadMeta {
    pub fn new(id: TID, os_thread: Thread) -> Self {
        Self {
            id,
            os_thread,
            interrupted: RwLock::new(false),
            state: RwLock::new(ThreadState::Running),
            unsafe_unpark_count: Mutex::new(0),
        }
    }

    pub fn block(&self) {
        *self.state.write() = ThreadState::Blocked
    }
    pub fn unblock(&self) {
        *self.state.write() = ThreadState::Running
    }

    pub fn sleep(&self) {
        *self.state.write() = ThreadState::Sleeping
    }
    pub fn woken(&self) {
        *self.state.write() = ThreadState::Running
    }

    pub fn wait(&self, associate: MonitorAssociate) {
        *self.state.write() = ThreadState::Waiting(associate)
    }
    pub fn notified(&self) {
        *self.state.write() = ThreadState::Running
    }

    pub fn park(&self) {
        *self.state.write() = ThreadState::Parked
    }
    pub fn unpark(&self) {
        *self.state.write() = ThreadState::Running
    }
}

impl PartialEq for ThreadMeta {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.os_thread.id() == other.os_thread.id()
    }
}

pub struct JavaThread {
    pub meta: Arc<ThreadMeta>,
    pub thread_obj_id: Option<RefId>,

    pub call_stack: CallStack,
    pub debug_helper: DebugHelper,
    pub caught_exception: RefCell<Option<(String, String, Value)>>,

    pub jni_env: Pin<Box<JNIEnv>>,
    pub java_vm: Pin<Box<JavaVM>>,
}

impl JavaThread {
    pub fn new(id: TID) -> Self {
        Self {
            meta: Arc::new(ThreadMeta::new(id, std::thread::current())),
            thread_obj_id: None,
            call_stack: CallStack::new(),
            debug_helper: DebugHelper::new(),
            caught_exception: RefCell::new(None),

            jni_env: Box::pin(JNIEnv::new(std::ptr::null())),
            java_vm: Box::pin(JavaVM::new()),
        }
    }

    pub fn invoke_subroutine<'a>(ctx: Context<'a, '_>, class_and_method: ClassAndMethod<'a>, object: Option<Reference<'a>>, args: Vec<Value>) -> VMPartialResult<Option<Value>>{
        let current_index = ctx.thread.call_stack.len() as isize -1;
        ctx.create_and_push_call_frame(class_and_method, object, args, false);
        Self::invoke_frames_until(ctx, current_index)
    }

    pub fn thread_entry<'a>(ctx: Context<'a, '_>, camid: ClassAndMethodId, obj_id: RefId, args: Vec<Value>) -> VMResult<()> {
        let current_index = ctx.thread.call_stack.len() as isize -1;
        let class_and_method = ClassAndMethod::try_resolve(ctx.vm, &camid)?;
        let obj = ctx.vm.resolve_object_by_id(obj_id)?;
        ctx.create_and_push_call_frame(class_and_method, Some(obj), args, false);
        let VMResultType::Successful(None) = Self::invoke_frames_until(ctx, current_index)? else { return Err(VmError::Unspecified("Thread exited unsuccessfully".to_owned())) };
        Ok(())
    }

    /// Returns only Err() or Ok(Successful())
    pub fn invoke_frames_until<'a>(ctx: Context<'a, '_>, stop_index: isize) -> VMPartialResult<Option<Value>> {
        loop {
            let frame_amount = ctx.thread.call_stack.len();

            // if an exception is caught, try to let the current frame handle it
            let mut clear_exception = false;
            if let Some((message, origin, Value::Reference(throwable_ref_id))) = ctx.thread.caught_exception.borrow().as_ref(){
                let thrown_class_name = ctx.vm.resolve_object_by_id(*throwable_ref_id)?.class_name.clone();
                if frame_amount as isize - 1 == stop_index {
                    ctx.thread.debug_helper.exception_helper.push(format!("Subroutine could not handle {} thrown by function {} with message: {}", thrown_class_name, origin, message));
                    return Err(VmError::JavaException(JavaError::JavaExceptionThrown(thrown_class_name, message.to_owned(), origin.to_owned())));
                }

                let camid = ctx.thread.call_stack.get_class_and_method_id_cloned();
                let class_and_method = ClassAndMethod::try_resolve(ctx.vm, &camid)?;
                if class_and_method.method.is_native(){
                    ctx.thread.call_stack.pop_call_frame();
                    debug!("Exception handler not in this native function {}", class_and_method.format());
                    continue;
                }
                let current_pc = &ctx.thread.call_stack.get_pc();
                //[unchecked] class already loaded by method
                if let Some(handler_pc) = class_and_method.resolve_exception_handler(&ctx, current_pc, thrown_class_name.as_str()){
                    ctx.thread.call_stack.set_pc(handler_pc);
                    ctx.thread.call_stack.clear_operand_stack();
                    ctx.thread.call_stack.push_operand_value(Value::Reference(*throwable_ref_id));
                    ctx.thread.debug_helper.exception_helper.push(format!("Handled {} by {}\n└-- thrown by {} with message: {}", thrown_class_name, class_and_method.format(), origin, message));
                    debug!("Exception thrown handled by {}", class_and_method.format());
                    clear_exception = true;
                } else {
                    ctx.thread.call_stack.pop_call_frame();
                    debug!("Exception handler not in this function {}", class_and_method.format());
                    continue;
                }
            }

            let camid = ctx.thread.call_stack.get_class_and_method_id_cloned();
            let class_and_method = ClassAndMethod::try_resolve(ctx.vm, &camid)?;
            if clear_exception {
                ctx.thread.caught_exception.replace(None);
            }

            let call_result = if class_and_method.method.is_native(){
                Self::execute_native(ctx, class_and_method)?
            } else {
                executor::execute(ctx)?
            };

            match call_result {
                // borde alltid och bara vara på return av non-native och native funktioner
                // så den här frame är alltid den översta
                VMResultType::Successful(result) => {
                    let frame = ctx.thread.call_stack.pop_call_frame();
                    if frame_amount as isize -2 == stop_index{
                        return Ok(VMResultType::Successful(result));
                    }
                    if let Some(value) = result{
                        if frame.should_push_return{
                            ctx.thread.call_stack.push_operand_value(value);
                        }
                    }
                }
                // returned by both non-native and native functions
                VMResultType::ExceptionThrown => {
                    // thrown exception should be in self.caught_exception
                    // nothing more to do here
                    continue;
                }
                // should only be returned by non-native functions
                VMResultType::Interrupted(frame_amount, reset_pc) => {
                    if reset_pc{
                        let last_frame_index = ctx.thread.call_stack.pcs.borrow().len() - frame_amount - 1;
                        let current_pc = ctx.thread.call_stack.pcs.borrow()[last_frame_index];
                        let camid = ctx.thread.call_stack.frames.borrow()[last_frame_index].class_and_method;
                        let cam = ClassAndMethod::try_resolve(ctx.vm, &camid)?;
                        let previous_pc = cam.method.previous_pc(current_pc);
                        *ctx.thread.call_stack.pcs.borrow_mut().get_mut(last_frame_index).unwrap() = ProgramCounter(previous_pc);
                    }
                }
            }
        }
    }

    fn execute_native<'a>(ctx: Context<'a, '_>, class_and_method: ClassAndMethod<'a>) -> VMPartialResult<Option<Value>> {
        //let call_frame = self.call_stack.pop_call_frame();

        let object = if class_and_method.method.is_static() {
            None
        } else {
            match ctx.thread.call_stack.load_local(0) {
                Some(Value::Reference(local_id)) => {
                    let local_ref = if local_id.is_null() {
                        ctx.vm.null_ref()
                    } else {
                        ctx.vm.resolve_object_by_id(local_id)?
                    };
                    Some(local_ref)
                },
                None => None,
                _ => return Err(VmError::ValidationError("Expected a reference".to_owned()))
            }
        };
        let args = ctx.thread.call_stack.locals_stack.borrow().last().unwrap()
            .iter()
            .cloned()
            .skip(if object.is_none() {0} else {1})
            .take_while(|value| value != &Value::Uninitialized)
            .collect::<Vec<_>>();
        let try_native = NativeMethodRegistry::invoke(ctx, &class_and_method, object, args);
        debug!("TTT native[{}] returned: {:?}", class_and_method.format(), try_native);
        if let Some(native) = try_native {
            native
        } else {
            debug!("native not found");
            if class_and_method.method.descriptor.return_type.is_some(){
                Err(VmError::MethodCallError(format!("native {} returns a value which is probably used", class_and_method.format())))
            } else {
                warn!(target: "native", "Native function: {} not found. Skipping", class_and_method.format());
                Ok(VMResultType::Successful(None))
            }
        }
    }

    /// Returns a `VMResultType::ExceptionThrown` and places the throwable into the exception slot
    ///
    /// `throwable_class` has to be initialized beforehand
    ///
    pub fn throw<'a, T>(ctx: Context<'a, '_>, throwable_class: ClassRef<'a>, message: String, origin: String) -> VMPartialResult<T> {
        //let exception_class = self.get_or_initialize_class(&throwable_class_name)?;
        let exception_object = ctx.new_object_from_class(throwable_class);

        let details = ctx.try_new_string_object(message.as_str())?;
        //detailsMessage
        exception_object.set_field(THROWABLE_detailsMessage_INDEX, Value::Reference(details.id));

        let prev = ctx.thread.caught_exception.replace(
            Some((
                message,
                origin,
                Value::Reference(exception_object.id)
            )));
        assert!(prev.is_none());
        Ok(VMResultType::ExceptionThrown)
    }
}

impl !Unpin for JavaThread {}