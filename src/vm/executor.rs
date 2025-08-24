use crate::vm::{bytecode::InstructionBlock, class::ClassAndMethod, result::VMPartialResult, value::Value, VmError, VM};

pub fn execute<'a>(vm: &mut VM, class_and_method: ClassAndMethod) -> VMPartialResult<'a, Option<Value<'a>>>{
    if let Some(code) = &class_and_method.method.code{
        let mut result = execute_block(vm, &class_and_method, class_and_method.method.get_code_block_at(vm.call_stack.get_pc()));
        while let None = result{
            
            result = execute_block(vm, &class_and_method, class_and_method.method.get_code_block_at(vm.call_stack.get_pc()));
        }
        return result.unwrap();
    }
    Err(VmError::MethodCallError(format!("Method: {} is not executeable, because it has no code", class_and_method.format())))
}

pub fn execute_block<'a>(vm: &mut VM, class_and_method: &ClassAndMethod, block: &InstructionBlock) -> Option<VMPartialResult<'a, Option<Value<'a>>>>{
    None
}