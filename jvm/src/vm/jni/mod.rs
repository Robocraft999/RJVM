pub mod jvm;
pub mod types;
pub mod env_function_table;
pub mod vm_function_table;

#[macro_export]
macro_rules! native_init_wrap {
    ($env:expr, $x:expr) => {
        {
            let macro_vm: &VM = unsafe{&*(*$env).vm};
            let macro_javavm: &JavaVM = unsafe{&*(*$env).pvm};
            let macro_current_frame_index: isize = macro_vm.call_stack.len() as isize -1;
            let macro_res = $x;
            match macro_res.unwrap(){
                VMResultType::Successful(v) => v,
                VMResultType::Interrupted(..) => {
                    let init_res = macro_vm.invoke_frames_until(macro_javavm, macro_current_frame_index).unwrap();
                    if let VMResultType::Successful(None) = init_res{
                        if let VMResultType::Successful(v) = ($x).unwrap(){
                            v
                        } else {
                            unreachable!("[wrap_init] still needs classes even after loading them")
                        }
                    } else {
                        unreachable!("[wrap_init] still classes to init after initting them")
                    }
                }
                other => unreachable!("[wrap_init] got unexpected: {:?}", other),
            }
        }
    }
}