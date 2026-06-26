use crate::vm::class::ClassAndMethod;
use std::fmt::Debug;

#[derive(Clone)]
pub struct CallFrame<'a> {
    pub class_and_method: ClassAndMethod<'a>,
    pub should_push_return: bool,
}

#[derive(Debug, PartialEq)]
enum InvokeKind {
    STATIC,
    SPECIAL,
    VIRTUAL,
    INTERFACE,
}