use crate::class_file::methods::{MethodInfo, NONVIRTUAL_VTABLE_INDEX};
use crate::vm::class::ClassRef;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CallInfoKind {
    Unknown,
    Direct,
    Vtable,
    Itable,
}

pub struct CallInfo<'a> {
    resolved_class: ClassRef<'a>,
    selected_class: ClassRef<'a>,
    resolved_method: &'a MethodInfo,
    selected_method: &'a MethodInfo,
    pub kind: CallInfoKind,
    pub index: isize,
}

impl<'a> CallInfo<'a>{
    // https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/interpreter/linkResolver.cpp#L137
    pub fn new(resolved_method: &'a MethodInfo, resolved_class: ClassRef<'a>) -> CallInfo<'a> {
        let mut kind = CallInfoKind::Unknown;
        let mut index = resolved_method.vtable_index();

        if resolved_method.is_final() || resolved_class.is_final(){
            kind = CallInfoKind::Direct;
        } else if !resolved_method.is_holder_interface{
            kind = CallInfoKind::Vtable;
        } else if !resolved_class.is_interface() {
            // default of miranda method
            // FIXME get this
            //index = <resolve_vtable_index_of_interface_method>
            kind = CallInfoKind::Vtable;
        } else if resolved_method.has_vtable_index() {
            // interface redeclares method of Object
            kind = CallInfoKind::Vtable;
        } else {
            // regular interface call
            kind = CallInfoKind::Itable;
            index = resolved_method.itable_index();
        }

        assert!(index == NONVIRTUAL_VTABLE_INDEX || index >= 0);

        Self {
            resolved_class,
            selected_class: resolved_class,
            resolved_method,
            selected_method: resolved_method,
            kind,
            index
        }
    }
}