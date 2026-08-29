use crate::vm::class::class_and_member::ClassAndMethodId;

#[derive(Clone)]
pub struct CallFrame {
    pub class_and_method: ClassAndMethodId,
    pub should_push_return: bool,
}