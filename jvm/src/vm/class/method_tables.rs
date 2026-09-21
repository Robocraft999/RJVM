use std::collections::HashMap;
use std::ops::Index;
use crate::class_file::methods::{MethodInfo, ITABLE_INDEX_MAX, NONVIRTUAL_VTABLE_INDEX, PENDING_ITABLE_INDEX};
use crate::vm::class::{Class, ClassId, ClassRef};

#[derive(Debug, Clone)]
pub struct VTable {
    table: Vec<MethodInfo>,
}

impl VTable {
    pub fn new() -> Self {
        Self {
            table: Vec::new()
        }
    }

    pub const fn len(&self) -> usize {
        self.table.len()
    }

    pub fn put_method_at(&mut self, method: MethodInfo, index: usize) {
        if self.len() == index {
            self.table.push(method)
        } else if index < self.len() {
            self.table[index] = method;
        } else {
            unreachable!()
        }
    }

    pub fn method_at(&self, index: usize) -> Option<&MethodInfo> {
        self.table.get(index)
    }

    pub fn index_of(&self, method: &MethodInfo) -> Option<isize> {
        if !method.has_vtable_index() {
            None
        } else {
            Some(method.vtable_index())
        }
    }

    pub fn update_inherited_vtable<>(&mut self, clazz: &Class<'_>, target_method: &mut MethodInfo, super_vtable_len: usize, default_index: isize) -> bool {
        let clazz_final = clazz.is_final();
        let clazz_interface = clazz.is_interface();

        let mut allocate_new = true;
        let mut is_default = false;

        if default_index >= 0 {
            is_default = true;

        } else {
            target_method.vtable_index = NONVIRTUAL_VTABLE_INDEX;
        }

        if target_method.is_static() || target_method.name == "<init>" {
            return false;
        }

        if target_method.is_final() || clazz_final {
            allocate_new = false;
        } else if clazz_interface {
            allocate_new = false;

            if !is_default || !target_method.has_itable_index() {
                target_method.vtable_index = PENDING_ITABLE_INDEX;
            }
        }

        if clazz.superclass.is_none() {
            return allocate_new;
        }

        if target_method.is_private() {
            return allocate_new;
        }

        for i in 0..super_vtable_len {
            let super_method = self.method_at(i).unwrap();

            if super_method == target_method {
                if !super_method.is_private() && (is_default || false) {
                    if !target_method.is_package_private() {
                        allocate_new = false;
                    }
                }

                if !is_default {
                    target_method.vtable_index = i as isize;
                } else {

                }
                self.put_method_at(target_method.clone(), i);
            }
        }

        allocate_new
    }

    pub fn initialize(clazz: &mut Class<'_>) -> VTable{
        let mut vtable = match clazz.superclass {
            None => VTable::new(),
            Some(super_clazz) => super_clazz.vtable.clone(),
        };
        let super_vtable_len = vtable.len();

        if clazz.is_array() {
            // should not introduce new methods
        } else {
            let len = clazz.methods.len();
            let mut initialized = super_vtable_len;

            for i in 0..len {
                let mut target_method = clazz.methods.get(i).cloned().unwrap();
                let potential_new_entry = vtable.update_inherited_vtable(clazz, &mut target_method, super_vtable_len, -1);

                if potential_new_entry {
                    target_method.vtable_index = initialized as isize;
                    vtable.put_method_at(target_method.clone(), initialized);
                    initialized += 1;
                }

                clazz.methods[i] = target_method;
            }
        }

        vtable
    }
}

#[derive(Debug, Clone)]
pub struct ITable {
    entries: Vec<Vec<Option<MethodInfo>>>,
    interface_to_index: HashMap<ClassId, usize>,
}

impl ITable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            interface_to_index: HashMap::new(),
        }
    }

    pub const fn entries(&self) -> usize {
        self.entries.len()
    }

    pub fn initialize(clazz: &mut Class<'_>) -> ITable {
        let mut itable = ITable::new();
        if clazz.is_interface() {
            let len = clazz.methods.len();
            let mut ime_num = 0;

            for i in 0..len {
                let mut target_method = clazz.methods.get(i).cloned().unwrap();

                if !target_method.is_static() && !target_method.is_initializer() {
                    assert!(!target_method.is_final());

                    if !target_method.has_vtable_index() {
                        assert_eq!(PENDING_ITABLE_INDEX, target_method.vtable_index());

                        target_method.vtable_index = ITABLE_INDEX_MAX - ime_num;
                        ime_num += 1;

                        clazz.methods[i] = target_method;
                    }
                }
            }
        }

        let transitive_interfaces = clazz.transitive_interfaces();
        let num_interfaces = transitive_interfaces.len();
        itable.entries = vec![Vec::new(); num_interfaces];

        for i in 0..num_interfaces {
            itable.initialize_for_interface(clazz, i, &transitive_interfaces[i])
        }

        itable
    }

    fn initialize_for_interface(&mut self, clazz: &Class<'_>, entry_offset: usize, interface_clazz: ClassRef) {
        self.interface_to_index.insert(interface_clazz.id, entry_offset);
        let mut methods = vec![None; interface_clazz.methods.len()];

        let nof_methods = interface_clazz.methods.len();
        for i in 0..nof_methods {
            let mut m = interface_clazz.methods.get(i).cloned().unwrap();
            let target = if m.has_itable_index() {
                clazz.lookup_instance_method(m.name.as_str(), m.descriptor.as_str())
            } else {
                None
            };

            if let Some(target) = target {
                if !target.is_public() || target.is_abstract() {
                    // if not public -> fill with illegal access error
                } else {
                    let ime_num = m.itable_index() as usize;
                    methods[ime_num] = Some(target)
                }
            }
        }

        self.entries[entry_offset] = methods;
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::application::Application;
    use crate::vm::class_path::ClassPath;

    #[test]
    fn test_default_interface_method() {
        let mut cp = ClassPath::default();
        cp.push("../resources/rt.jar;../resources/test").unwrap();
        let mut app = Application::new(cp);

        app.startup();

        app.start_test("InterfaceDefaultTest");
    }
}