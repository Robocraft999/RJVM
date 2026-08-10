use crate::vm::r#unsafe::memory_chunk::MemoryChunk;
use crate::vm::result::VMResult;
use parking_lot::RwLock;
use crate::vm::jni::types::{jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};

mod memory_chunk;

macro_rules! gen_mem_access {
    ($name_get:ident, $name_put:ident, $simple_type:ty, $native_type:ty) => {
        pub fn $name_put(&self, ptr: i64, value: $simple_type) -> VMResult<()> {
            let mut guard = self.memory.write();
            guard.put(ptr as u64, size_of::<$native_type>(), &(value as $native_type).to_le_bytes())
        }

        pub fn $name_get(&self, ptr: i64) -> VMResult<$simple_type> {
            let guard = self.memory.read();
            let bytes = guard.get(ptr as u64, size_of::<$native_type>())?;
            let value = <$native_type>::from_le_bytes(bytes.try_into().unwrap()) as $simple_type;
            Ok(value)
        }
    };
}

pub struct Unsafe {
    memory: RwLock<MemoryChunk>
}

impl Unsafe {
    pub fn new() -> Self {
        Self{
            memory: RwLock::new(MemoryChunk::new(1024 * 1024 * 50))
        }
    }

    pub fn allocate_memory(&self, amount: usize) -> i64 {
        let mut guard = self.memory.write();
        guard.alloc(amount).map(|entry| entry.ptr as i64).unwrap_or(-1)
    }

    pub fn set_memory(&self, ptr: i64, amount: usize, byte: u8) -> VMResult<()> {
        let mut guard = self.memory.write();
        guard.put(ptr as u64, amount, &vec![byte; amount])
    }

    pub fn copy_memory(&self, src: i64, dst: i64, amount: usize) -> VMResult<()> {
        let guard = self.memory.write();
        guard.copy(src as u64, dst as u64, amount)
    }

    pub fn free_memory(&self, ptr: i64) {
        //TODO
    }

    gen_mem_access!(get_byte, put_byte, i32, jbyte);
    gen_mem_access!(get_short, put_short, i32, jshort);
    gen_mem_access!(get_char, put_char, i32, jchar);
    gen_mem_access!(get_int, put_int, i32, jint);
    gen_mem_access!(get_long, put_long, i64, jlong);
    gen_mem_access!(get_float, put_float, f32, jfloat);
    gen_mem_access!(get_double, put_double, f64, jdouble);

    pub fn put_bytes(&self, ptr: i64, bytes: &[u8]) -> VMResult<()> {
        let mut guard = self.memory.write();
        guard.put(ptr as u64, bytes.len(), bytes)
    }

    pub fn get_bytes(&self, ptr: i64, length: usize) -> VMResult<Vec<u8>> {
        let guard = self.memory.read();
        Ok(guard.get(ptr as u64, length)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::r#unsafe::Unsafe;

    #[test]
    fn test_allocate_memory() {
        let unsafe_allocator = Unsafe::new();
        assert_ne!(unsafe_allocator.allocate_memory(8), -1)
    }

    #[test]
    fn test_long(){
        let unsafe_allocator = Unsafe::new();
        let offset = unsafe_allocator.allocate_memory(8);
        unsafe_allocator.put_long(offset, 12).unwrap();
        assert_eq!(unsafe_allocator.get_long(offset).unwrap(), 12);
    }

    #[test]
    fn test_long_and_byte(){
        let unsafe_allocator = Unsafe::new();
        let offset = unsafe_allocator.allocate_memory(8);
        //bytes get stored as little endian (See put_long), so we expect 8 (See java/nio/Bits.<clinit>)
        unsafe_allocator.put_long(offset, 72623859790382856).unwrap();
        assert_eq!(unsafe_allocator.get_byte(offset).unwrap(), 8);
    }
}