use crate::vm::result::VMResult;
use log::{debug, warn};
use std::alloc::Layout;
use std::fmt;
use std::fmt::Formatter;

pub struct MemoryChunk {
    start: u64,
    memory: *mut u8,
    used: usize,
    capacity: usize,
}

impl fmt::Debug for MemoryChunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{address={:#0x}, used={}, capacity={}}}", self.memory as u64, self.used, self.capacity)
    }
}

unsafe impl Sync for MemoryChunk { }
unsafe impl Send for MemoryChunk { }

impl MemoryChunk {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        debug!(
            "allocated memory chunk of size {} at {:#0x}",
            capacity, ptr as u64
        );

        MemoryChunk {
            start: ptr as u64,
            memory: ptr,
            capacity,
            used: 0,
        }
    }

    pub fn alloc(&mut self, required_size: usize) -> Option<AllocEntry> {
        if self.used + required_size > self.capacity {
            return None;
        }

        // We require all allocations to be aligned to 8 bytes!
        let allocated_size = required_size + required_size % 8;
        assert_eq!(allocated_size % 8, 0);

        let ptr = unsafe { self.memory.add(self.used) };
        self.used += allocated_size;

        Some(AllocEntry {
            ptr,
            alloc_size: allocated_size,
        })
    }

    pub fn put(&mut self, ptr: u64, bytes: usize, data: &[u8]) -> VMResult<()>{
        //assert!(offset + data.len() <= self.capacity);
        if ptr < self.start || ptr + bytes as u64 > self.start + self.capacity as u64 {
            warn!("unsafe writing: Safe bounds are [{:#0x}-{:#0x}], ptr is: {:#0x}, writing {} bytes", self.start, self.start + self.capacity as u64, ptr, bytes)
        }
        unsafe {
            // std::ptr::copy(data.as_ptr(), ptr as *mut u8, bytes);
            for i in 0..bytes {
                std::ptr::write((ptr + i as u64) as *mut u8, data[i]);
            }
        }
        Ok(())
    }

    pub fn get(&self, ptr: u64, bytes: usize) -> VMResult<Vec<u8>> {
        if ptr < self.start || ptr + bytes as u64 > self.start + self.capacity as u64 {
            warn!("unsafe reading: Safe bounds are [{:#0x}-{:#0x}], ptr is: {:#0x}, reading {} bytes", self.start, self.start + self.capacity as u64, ptr, bytes)
        }
        unsafe {
            Ok(std::slice::from_raw_parts(ptr as *const u8, bytes).to_vec())
        }
    }
}

pub struct AllocEntry {
    pub(crate) ptr: *mut u8,
    pub(crate) alloc_size: usize,
}