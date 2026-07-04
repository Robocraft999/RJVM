use crate::class_file::fields::field_type::FieldType;
use crate::vm::class::{ClassId, ClassRef};
use crate::vm::value::{RefId, Reference, ReferenceType, ReferenceValue, Value};
use std::cell::RefCell;
use std::sync::{Mutex, RwLock};
use typed_arena::Arena;

pub struct ObjectAllocator<'a>{
    arena: Mutex<Arena<ReferenceValue>>,
    next_object_id: RwLock<u32>,
    pub null: Reference<'a>,
}

impl<'a> ObjectAllocator<'a>{
    pub(crate) fn new() -> Self{
        let arena = Arena::with_capacity(1024);
        let _null = arena.alloc(ReferenceValue{
            id: RefId(0),
            class_id: ClassId(u32::MAX),
            class_name: String::from("xXxNullxXx"),
            reference_type: ReferenceType::Object(RwLock::new(Vec::new())),
        });
        let null_ptr: *const ReferenceValue = _null;
        let null = unsafe{&*null_ptr};
        ObjectAllocator{
            arena: Mutex::new(arena),
            next_object_id: RwLock::new(1),
            null
        }
    }

    pub fn allocate_object(&self, class: ClassRef<'a>, fields: Vec<Value>) -> Reference<'a>{
        let Ok(mut current_id) = self.next_object_id.write() else { unreachable!("Could not acquire next object id lock") };
        let object = ReferenceValue{
            id: RefId(*current_id),
            class_id: class.id,
            class_name: class.name.to_string(),
            reference_type: ReferenceType::Object(RwLock::new(fields))
        };

        if let Ok(arena) = self.arena.lock() {
            let new_object = arena.alloc(object);

            *current_id += 1;
            unsafe {
                let object_ptr: *const ReferenceValue = new_object;
                &*object_ptr
            }
        } else { unreachable!("Could not acquire object lock") }
    }

    pub fn allocate_array(&self, class: ClassRef<'a>, dims: usize, component_type: FieldType, content: RwLock<Vec<Value>>) -> Reference<'a>{
        let Ok(mut current_id) = self.next_object_id.write() else { unreachable!("Could not acquire next object id lock") };
        let array = ReferenceValue{
            id: RefId(*current_id),
            class_id: class.id,
            class_name: class.name.to_string(),
            reference_type: ReferenceType::Array(dims, component_type, content),
        };

        if let Ok(arena) = self.arena.lock() {
            let new_object = arena.alloc(array);

            *current_id += 1;
            unsafe {
                let object_ptr: *const ReferenceValue = new_object;
                &*object_ptr
            }
        } else { unreachable!("Could not acquire object lock") }
    }
}