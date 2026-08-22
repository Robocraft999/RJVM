use parking_lot::Mutex;
use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::class::ClassRef;
use crate::vm::heap::array::{ArrayContent, RawArray};
use crate::vm::heap::memory_chunk::MemoryChunk;
use crate::vm::jni::types::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort};
use crate::vm::value::Value;

pub mod array;
mod memory_chunk;
pub mod direct;

struct Chunks {
    chunks: Vec<MemoryChunk>,
}

impl Chunks {
    pub fn new() -> Self {
        Self {
            chunks: vec![Self::new_chunk()],
        }
    }

    pub fn allocate(&mut self, amount: usize) -> u64 {
        match self.chunks.last_mut().unwrap().alloc(amount) {
            Some(entry) => entry.ptr as u64,
            None => {
                let mut chunk = Self::new_chunk();
                let ptr = chunk.alloc(amount).unwrap().ptr as u64;
                self.chunks.push(chunk);
                ptr
            }
        }
    }

    fn new_chunk() -> MemoryChunk {
        MemoryChunk::new(1024 * 1024 * 4)
    }
}

pub struct HeapAllocator {
    chunks: Mutex<Chunks>,
}

impl HeapAllocator {
    pub fn new() -> Self {
        Self {
            chunks: Mutex::new(Chunks::new()),
        }
    }

    fn allocate(&self, amount: usize) -> u64 {
        self.chunks.lock().allocate(amount)
    }

    fn alloc_native_arr<T>(&self, length: usize) -> RawArray<T>{
        let width = size_of::<T>();
        let ptr = self.allocate(length * width) as *mut T;
        RawArray::new(ptr, length)
    }

    pub fn allocate_array_body(&self, clazz: ClassRef, content: Vec<Value>) -> ArrayContent {
        if let Some(ingo) = &clazz.array_info {
            let length = content.len();
            if ingo.dims > 1 {
                let raw_array = self.alloc_native_arr::<jobject>(length);
                let native_values = content.iter().map(|v| if let Value::Reference(ref_id) = v { ref_id.nid() } else { unreachable!() }).collect::<Vec<_>>();
                unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                ArrayContent::Ref(raw_array)
            } else {
                match ingo.component_type {
                    FieldType::Primitive(PrimitiveType::Boolean) => {
                        let raw_array = self.alloc_native_arr::<jboolean>(length);
                        let native_values = content.iter().map(|v| if let Value::Integer(val) = v { *val as jboolean } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Bool(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Byte) => {
                        let raw_array = self.alloc_native_arr::<jbyte>(length);
                        let native_values = content.iter().map(|v| if let Value::Integer(val) = v { *val as jbyte } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Byte(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Char) => {
                        let raw_array = self.alloc_native_arr::<jchar>(length);
                        let native_values = content.iter().map(|v| if let Value::Integer(val) = v { *val as jchar } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Char(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Short) => {
                        let raw_array = self.alloc_native_arr::<jshort>(length);
                        let native_values = content.iter().map(|v| if let Value::Integer(val) = v { *val as jshort } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Short(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Integer) => {
                        let raw_array = self.alloc_native_arr::<jint>(length);
                        let native_values = content.iter().map(|v| if let Value::Integer(val) = v { *val as jint } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Int(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Long) => {
                        let raw_array = self.alloc_native_arr::<jlong>(length);
                        let native_values = content.iter().map(|v| if let Value::Long(val) = v { *val as jlong } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Long(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Float) => {
                        let raw_array = self.alloc_native_arr::<jfloat>(length);
                        let native_values = content.iter().map(|v| if let Value::Float(val) = v { *val as jfloat } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Float(raw_array)
                    }
                    FieldType::Primitive(PrimitiveType::Double) => {
                        let raw_array = self.alloc_native_arr::<jdouble>(length);
                        let native_values = content.iter().map(|v| if let Value::Double(val) = v { *val as jdouble } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Double(raw_array)
                    }
                    _ => {
                        let raw_array = self.alloc_native_arr::<jobject>(length);
                        let native_values = content.iter().map(|v| if let Value::Reference(ref_id) = v { ref_id.nid() } else { unreachable!() }).collect::<Vec<_>>();
                        unsafe { raw_array.unchecked_copy_from(native_values.as_ptr(), 0, length); }
                        ArrayContent::Ref(raw_array)
                    }
                }
            }
        } else {
            unreachable!("clazz has no array info")
        }
    }
}

