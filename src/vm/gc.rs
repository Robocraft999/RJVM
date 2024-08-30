use std::cell::RefCell;

use typed_arena::Arena;

use crate::vm::class::ClassRef;
use crate::vm::value::{ObjectRef, ObjectValue};

pub struct ObjectAllocator<'a>{
    arena: Arena<ObjectValue<'a>>
}

impl<'a> ObjectAllocator<'a>{
    pub(crate) fn new() -> Self{
        ObjectAllocator{
            arena: Arena::with_capacity(1024)
        }
    }

    pub fn allocate(&self, class: ClassRef<'a>) -> ObjectRef<'a>{
        let new_object = self.arena.alloc(self.object_from_class(class));
        unsafe {
            let object_ptr: *const ObjectValue = new_object;
            &*object_ptr
        }
    }

    fn object_from_class(&self, class: ClassRef<'a>) -> ObjectValue<'a>{
        let fields = class.get_fields();
        ObjectValue{
            id: class.id,
            fields: RefCell::new(fields)
        }
    }
}