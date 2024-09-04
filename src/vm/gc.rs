use std::cell::RefCell;
use std::ops::Add;
use typed_arena::Arena;

use crate::vm::class::ClassRef;
use crate::vm::value::{ObjectRef, ObjectValue};

pub struct ObjectAllocator<'a>{
    arena: Arena<ObjectValue<'a>>,
    next_object_id: RefCell<u32>,
}

impl<'a> ObjectAllocator<'a>{
    pub(crate) fn new() -> Self{
        ObjectAllocator{
            arena: Arena::with_capacity(1024),
            next_object_id: RefCell::new(0),
        }
    }

    pub fn allocate(&self, class: ClassRef<'a>) -> ObjectRef<'a>{
        let new_object = self.arena.alloc(self.object_from_class(class));
        *self.next_object_id.borrow_mut() += 1;
        unsafe {
            let object_ptr: *const ObjectValue = new_object;
            &*object_ptr
        }
    }

    fn object_from_class(&self, class: ClassRef<'a>) -> ObjectValue<'a>{
        let fields = class.get_fields();
        ObjectValue{
            id: *self.next_object_id.borrow(),
            class_id: class.id,
            fields: RefCell::new(fields)
        }
    }

    pub fn get_objects_count(&self) -> usize{
        self.arena.len()
    }
}