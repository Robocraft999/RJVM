use crate::vm::class::ClassAndMethodId;

#[derive(Clone)]
pub struct CallFrame {
    pub class_and_method: ClassAndMethodId,
    pub should_push_return: bool,
}