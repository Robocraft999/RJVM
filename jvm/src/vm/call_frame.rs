use crate::vm::class::{ClassAndMethod, ClassAndMethodId};
use std::fmt::Debug;

#[derive(Clone)]
pub struct CallFrame {
    pub class_and_method: ClassAndMethodId,
    pub should_push_return: bool,
}

#[derive(Debug, PartialEq)]
enum InvokeKind {
    STATIC,
    SPECIAL,
    VIRTUAL,
    INTERFACE,
}