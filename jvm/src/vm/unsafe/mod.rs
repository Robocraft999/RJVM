use crate::vm::r#unsafe::memory_chunk::MemoryChunk;
use crate::vm::result::VMResult;
use parking_lot::RwLock;

mod memory_chunk;

pub struct Unsafe {
    memory: RwLock<MemoryChunk>
}

impl Unsafe {
    pub fn new() -> Self {
        Self{
            memory: RwLock::new(MemoryChunk::new(1024 * 1024))
        }
    }

    pub fn allocate_memory(&self, amount: usize) -> i64 {
        let mut guard = self.memory.write();
        guard.alloc(amount).map(|entry| entry.ptr as i64).unwrap_or(-1)
    }

    pub fn free_memory(&self, ptr: i64) {
        //TODO
    }

    pub fn put_long(&self, ptr: i64, value: i64) -> VMResult<()> {
        let mut guard = self.memory.write();
        guard.put(ptr as u64, 8, &value.to_le_bytes())
    }

    pub fn get_long(&self, ptr: i64) -> VMResult<i64> {
        let guard = self.memory.read();
        let bytes = guard.get(ptr as u64, 8)?;
        let value = i64::from_le_bytes(bytes.try_into().unwrap());
        Ok(value)
    }

    pub fn put_int(&self, ptr: i64, value: i32) -> VMResult<()> {
        let mut guard = self.memory.write();
        guard.put(ptr as u64, 4, &value.to_le_bytes())
    }

    pub fn get_int(&self, ptr: i64) -> VMResult<i32> {
        let guard = self.memory.read();
        let bytes = guard.get(ptr as u64, 4)?;
        let value = i32::from_le_bytes(bytes.try_into().unwrap());
        Ok(value)
    }

    pub fn put_byte(&self, ptr: i64, value: u8) -> VMResult<()> {
        let mut guard = self.memory.write();
        guard.put(ptr as u64, 1, &[value])
    }

    pub fn get_byte(&self, ptr: i64) -> VMResult<u8> {
        let guard = self.memory.read();
        Ok(guard.get(ptr as u64, 1)?[0])
    }

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