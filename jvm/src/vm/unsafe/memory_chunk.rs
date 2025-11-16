use std::alloc::Layout;
use std::fmt;
use std::fmt::Formatter;
use log::debug;

pub struct MemoryChunk {
    memory: *mut u8,
    used: usize,
    capacity: usize,
}

impl fmt::Debug for MemoryChunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{{address={:#0x}, used={}, capacity={}}}", self.memory as u64, self.used, self.capacity)
    }
}

impl MemoryChunk {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        debug!(
            "allocated memory chunk of size {} at {:#0x}",
            capacity, ptr as u64
        );

        MemoryChunk {
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
        assert_eq!(required_size % 8, 0);

        let ptr = unsafe { self.memory.add(self.used) };
        self.used += required_size;

        Some(AllocEntry {
            ptr,
            alloc_size: required_size,
        })
    }

    pub fn put(&mut self, offset: usize, bytes: usize, data: &[u8]) {
        //assert!(offset + data.len() <= self.capacity);
        unsafe {
            for i in 0..bytes {
                std::ptr::write((offset + i * 8) as *mut u8, data[i]);
            }
        }
    }

    pub fn get(&self, offset: usize, bytes: usize) -> Vec<u8> {
        unsafe {
            let mut bytes_vec = Vec::with_capacity(bytes);
            for i in 0..bytes {
                bytes_vec.push(std::ptr::read((offset + i * 8) as *const u8));
            }
            bytes_vec
        }
    }
}

pub struct AllocEntry {
    pub(crate) ptr: *mut u8,
    pub(crate) alloc_size: usize,
}