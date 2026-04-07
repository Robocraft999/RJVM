use crate::class_file::fields::field_type::FieldType;
use crate::vm::class::{ClassId, ClassRef};
use crate::vm::value::{Reference, ReferenceType, ReferenceValue, Value};
use std::cell::RefCell;
use typed_arena::Arena;

pub struct ObjectAllocator<'a>{
    arena: Arena<ReferenceValue<'a>>,
    next_object_id: RefCell<u32>,
    pub null: Reference<'a>,
}

impl<'a> ObjectAllocator<'a>{
    pub(crate) fn new() -> Self{
        let arena = Arena::with_capacity(1024);
        let _null = arena.alloc(ReferenceValue{
            id: 0,
            class_id: ClassId(u32::MAX),
            class_name: String::from("xXxNullxXx"),
            reference_type: ReferenceType::Object(RefCell::new(Vec::new())),
        });
        let null_ptr: *const ReferenceValue = _null;
        let null = unsafe{&*null_ptr};
        ObjectAllocator{
            arena,
            next_object_id: RefCell::new(1),
            null
        }
    }

    pub fn allocate_object(&self, class: ClassRef<'a>, fields: Vec<Value<'a>>) -> Reference<'a>{
        let object = ReferenceValue{
            id: *self.next_object_id.borrow(),
            class_id: class.id,
            class_name: class.name.to_string(),
            reference_type: ReferenceType::Object(RefCell::new(fields))
        };

        let new_object = self.arena.alloc(object);
        *self.next_object_id.borrow_mut() += 1;
        unsafe {
            let object_ptr: *const ReferenceValue = new_object;
            &*object_ptr
        }
    }

    pub fn allocate_array(&self, class: ClassRef<'a>, dims: usize, component_type: FieldType, content: RefCell<Vec<Value<'a>>>) -> Reference<'a>{
        let array = ReferenceValue{
            id: *self.next_object_id.borrow(),
            class_id: class.id,
            class_name: class.name.to_string(),
            reference_type: ReferenceType::Array(dims, component_type, content),
        };

        let new_object = self.arena.alloc(array);
        *self.next_object_id.borrow_mut() += 1;
        unsafe {
            let object_ptr: *const ReferenceValue = new_object;
            &*object_ptr
        }
    }

    pub fn get_objects_count(&self) -> usize{
        self.arena.len()
    }
}