use crate::vm::r#unsafe::memory_chunk::MemoryChunk;
use std::cell::RefCell;

mod memory_chunk;

pub struct Unsafe {
    memory: RefCell<MemoryChunk>
}

impl Unsafe {
    pub fn new() -> Self {
        Self{
            memory: RefCell::new(MemoryChunk::new(1024 * 1024))
        }
    }

    pub fn allocate_memory(&self, amount: usize) -> i64 {
        self.memory.borrow_mut().alloc(amount).map(|entry| entry.ptr as i64).unwrap_or(-1)
    }

    pub fn free_memory(&self, ptr: i64) {
        //TODO
    }

    pub fn put_long(&self, ptr: i64, value: i64) {
        self.memory.borrow_mut().put(ptr as usize, 8, &value.to_be_bytes())
    }

    pub fn get_long(&self, ptr: i64) -> Option<i64> {
        let mut value: i64 = 0;
        for (index, element) in self.memory.borrow().get(ptr as usize, 8).iter().enumerate(){
            value |= (*element as i64) << (8 * (7-index));
        }
        Some(value)
    }

    pub fn put_byte(&self, ptr: i64, value: u8) {
        self.memory.borrow_mut().put(ptr as usize, 1, &[value])
    }

    pub fn get_byte(&self, ptr: i64) -> Option<u8> {
        Some(self.memory.borrow_mut().get(ptr as usize, 1)[0])
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::r#unsafe::Unsafe;

    #[test]
    fn test_allocate_memory() {
        let mut unsafe_allocator = Unsafe::new();
        assert_ne!(unsafe_allocator.allocate_memory(8), -1)
    }

    #[test]
    fn test_long(){
        let mut unsafe_allocator = Unsafe::new();
        let offset = unsafe_allocator.allocate_memory(8);
        unsafe_allocator.put_long(offset, 12);
        assert_eq!(unsafe_allocator.get_long(offset), Some(12));
    }

    #[test]
    fn test_long_and_byte(){
        let mut unsafe_allocator = Unsafe::new();
        let offset = unsafe_allocator.allocate_memory(8);
        //bytes get stored as big endian (See put_long), so we expect 1 (See java/nio/Bits.<clinit>)
        unsafe_allocator.put_long(offset, 72623859790382856);
        assert_eq!(unsafe_allocator.get_byte(offset), Some(1));
    }
}