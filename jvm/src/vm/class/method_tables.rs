use crate::class_file::methods::MethodInfo;
use crate::vm::class::Class;

#[derive(Debug, Clone)]
pub struct VTable<'a> {
    table: Vec<&'a MethodInfo>
}

impl<'a> VTable<'a> {
    pub fn new() -> Self {
        Self {
            table: Vec::new()
        }
    }

    pub const fn len(&self) -> usize {
        self.table.len()
    }

    pub fn put_method_at(&mut self, method: &'a MethodInfo, index: usize) {
        if self.len() == index {
            self.table.push(method)
        } else if self.len() < index {
            self.table[index] = method;
        } else {
            unreachable!()
        }
    }

    pub fn method_at(&self, index: usize) -> Option<&MethodInfo> {
        self.table.get(index).copied()
    }

    pub fn index_of(&self, method: &MethodInfo) -> Option<isize> {
        if !method.has_vtable_index() {
            None
        } else {
            Some(method.vtable_index())
        }
    }

    pub fn update_inherited_vtable(&mut self, clazz: &mut Class<'a>, super_vtable_len: usize, default_index: isize) -> bool {


        false
    }

    pub fn initialize(clazz: &mut Class<'a>) -> VTable<'a>{
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
                let method = clazz.methods.get(i).unwrap();
                let needs_new_entry = vtable.update_inherited_vtable(clazz, super_vtable_len, -1);

                if needs_new_entry {
                    vtable.put_method_at(method, initialized)
                }
            }
        }

        VTable::new()
    }
}