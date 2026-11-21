use crate::class_file::methods::{INVALID_VTABLE_INDEX, MethodInfo, NONVIRTUAL_VTABLE_INDEX};
use crate::vm::class::ClassRef;
use std::fmt::{Debug, Formatter};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CallInfoKind {
    Unknown,
    Direct,
    Vtable,
    Itable,
}

pub struct CallInfo<'a> {
    resolved_class: ClassRef<'a>,
    pub selected_class: ClassRef<'a>,
    pub resolved_method: &'a MethodInfo,
    pub selected_method: &'a MethodInfo,
    pub kind: CallInfoKind,
    pub index: isize,
}

impl<'a> CallInfo<'a>{
    // https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/interpreter/linkResolver.cpp#L137
    pub fn new(resolved_method: &'a MethodInfo, resolved_class: ClassRef<'a>) -> CallInfo<'a> {
        let mut kind = CallInfoKind::Unknown;
        let mut index = resolved_method.vtable_index();

        if can_be_statically_bound(resolved_method, resolved_class){
            kind = CallInfoKind::Direct;
        } else if !resolved_method.is_holder_interface{
            kind = CallInfoKind::Vtable;
        } else if !resolved_class.is_interface() {
            // default of miranda method
            // FIXME get this
            //index = <resolve_vtable_index_of_interface_method>
            assert!(index >= 0);
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

    pub fn new_static(resolved_class: ClassRef<'a>, resolved_method: &'a MethodInfo) -> CallInfo<'a> {
        Self {
            resolved_class,
            selected_class: resolved_class,
            resolved_method,
            selected_method: resolved_method,
            kind: CallInfoKind::Direct,
            index: NONVIRTUAL_VTABLE_INDEX,
        }
    }

    pub fn new_virtual(resolved_class: ClassRef<'a>, selected_class: ClassRef<'a>, resolved_method: &'a MethodInfo, selected_method: &'a MethodInfo, index: isize) -> CallInfo<'a> {
        assert!(index >= 0 || index == NONVIRTUAL_VTABLE_INDEX);
        assert!(index < 0 || !resolved_method.has_vtable_index() || index == resolved_method.vtable_index());
        let kind = if index >= 0 && !can_be_statically_bound(resolved_method, resolved_class){ CallInfoKind::Vtable } else { CallInfoKind::Direct };
        Self {
            resolved_class,
            selected_class,
            resolved_method,
            selected_method,
            kind,
            index,
        }
    }

    pub fn new_interface(resolved_class: ClassRef<'a>, selected_class: ClassRef<'a>, resolved_method: &'a MethodInfo, selected_method: &'a MethodInfo, index: isize) -> CallInfo<'a> {
        assert!(resolved_class.is_interface());
        assert_eq!(index, resolved_method.itable_index());
        Self {
            resolved_class,
            selected_class,
            resolved_method,
            selected_method,
            kind: CallInfoKind::Itable,
            index,
        }
    }

    pub fn set_common(&mut self, resolved_class: ClassRef<'a>, selected_class: ClassRef<'a>, resolved_method: &'a MethodInfo, selected_method: &'a MethodInfo, kind: CallInfoKind, index: isize) {
        assert_eq!(resolved_method.descriptor, selected_method.descriptor);

        self.resolved_class = resolved_class;
        self.selected_class = selected_class;
        self.resolved_method = resolved_method;
        self.selected_method = selected_method;
        self.kind = kind;
        self.index = index;
    }
}

impl Debug for CallInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallInfo")
            .field("kind", &self.kind)
            .field("resolved", &format_args!("{}.{}{}", self.resolved_class.name, self.resolved_method.name, self.resolved_method.descriptor.as_str()))
            .field("selected", &format_args!("{}.{}{}", self.selected_class.name, self.selected_method.name, self.selected_method.descriptor.as_str()))
            .field("index", &self.index)
            .finish()
    }
}

pub fn can_be_statically_bound<'a>(resolved_method: &'a MethodInfo, resolved_class: ClassRef<'a>) -> bool {
    if resolved_method.is_final() || resolved_class.is_final() { return true; }
    let is_non_virtual = resolved_method.vtable_index() == NONVIRTUAL_VTABLE_INDEX;
    if resolved_class.is_interface(){
        assert_eq!(is_non_virtual, resolved_method.is_static());
    }
    is_non_virtual
}

pub fn resolve_virtual_call<'a>(receiver_class: ClassRef<'a>, resolved_class: ClassRef<'a>, method_name: &str, method_signature: &str /*, caller:  ClassRef<'a>*/) -> CallInfo<'a> {
    let cam = resolved_class.resolve_method_virtual(method_name, method_signature).unwrap();
    let resolved_method = cam.method;

    let mut vtable_index = INVALID_VTABLE_INDEX;

    let selected_method = if resolved_method.is_holder_interface {
        //vtable_index = <vtable_index_of_interface_method>
        assert!(vtable_index >= 0);
        // FIXME get_method_at_vtable
        receiver_class.get_method_in_slot(vtable_index as usize).unwrap()
    } else {
        assert!(!resolved_method.has_itable_index());
        vtable_index = resolved_method.vtable_index();

        if vtable_index == NONVIRTUAL_VTABLE_INDEX {
            assert!(can_be_statically_bound(resolved_method, resolved_class));
            cam
        } else {
            // FIXME get_method_at_vtable
            receiver_class.get_method_in_slot(vtable_index as usize).unwrap()
        }
    };
    CallInfo::new_virtual(resolved_class, receiver_class, resolved_method, selected_method.method, vtable_index)
}

pub fn resolve_virtual_call2<'a>(receiver_class: ClassRef<'a>, resolved_class: ClassRef<'a>, resolved_method: &'a MethodInfo) -> Option<CallInfo<'a>> {
    if resolved_class.is_interface() {
        assert!(resolved_method.has_itable_index());
        let vtable_index = resolved_method.itable_index();
        assert!(vtable_index >= 0);
        let selected_method = receiver_class.get_itable_entry(&resolved_class.id, vtable_index as usize)?;
        // found a default method so the defined method should be in the itable of the resolved class
        Some(CallInfo::new_interface(resolved_class, resolved_class, resolved_method, selected_method, vtable_index))
    } else {
        assert!(!resolved_method.has_itable_index());
        let vtable_index = resolved_method.vtable_index();

        let selected_method = if vtable_index == NONVIRTUAL_VTABLE_INDEX {
            assert!(can_be_statically_bound(resolved_method, resolved_class));
            resolved_method
        } else {
            receiver_class.get_vtable_entry(vtable_index as usize)?
        };
        Some(CallInfo::new_virtual(resolved_class, receiver_class, resolved_method, selected_method, vtable_index))
    }
}

pub fn resolve_special_call<'a>(resolved_class: ClassRef<'a>, method_name: &str, method_signature: &str) -> CallInfo<'a> {
    let cam = if !resolved_class.is_interface() {
        resolved_class.resolve_method_virtual(method_name, method_signature).unwrap()
    } else {
        resolved_class.resolve_interface_method_virtual(method_signature, method_signature).unwrap()
    };
    let resolved_method = cam.method;

    //TODO more checks see (https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/interpreter/linkResolver.cpp#L908)

    let selected_method = resolved_method;

    CallInfo::new_static(resolved_class, selected_method)
}

/*
LinkResolver::resolve_special_call(CallInfo& result, Handle recv, KlassHandle resolved_klass, Symbol* method_name,
                                        Symbol* method_signature, KlassHandle current_klass, bool check_access, TRAPS)
LinkResolver::resolve_virtual_call(CallInfo& result, Handle recv, KlassHandle receiver_klass, KlassHandle resolved_klass,
                                        Symbol* method_name, Symbol* method_signature, KlassHandle current_klass,
                                        bool check_access, bool check_null_and_abstract, TRAPS)
 */

pub fn resolve_interface_call<'a>(receiver_class: ClassRef<'a>, resolved_class: ClassRef<'a>, resolved_method: &'a MethodInfo) -> Option<CallInfo<'a>> {
    assert!(resolved_class.is_interface() && !resolved_method.is_private());

    if !resolved_method.has_itable_index() {
        let vtable_index = resolved_method.vtable_index();
        let selected_method = receiver_class.get_vtable_entry(vtable_index as usize)?;
        assert_eq!(vtable_index, selected_method.vtable_index);
        Some(CallInfo::new_virtual(resolved_class, receiver_class, resolved_method, selected_method, vtable_index))
    } else {
        let vtable_index = resolved_method.itable_index();
        let selected_method = receiver_class.get_itable_entry(&resolved_class.id, vtable_index as usize)?;

        // FIXME selected method should be actual receiver
        if !selected_method.has_itable_index() {
            Some(CallInfo::new_virtual(resolved_class, receiver_class, resolved_method, selected_method, vtable_index))
        } else {
            Some(CallInfo::new_interface(resolved_class, resolved_class, resolved_method, selected_method, vtable_index))
        }
    }
}

pub fn resolve_static_call<'a>(resolved_class: ClassRef<'a>, resolved_method: &'a MethodInfo) -> Option<CallInfo<'a>> {
    Some(CallInfo::new_static(resolved_class, resolved_method))
}
