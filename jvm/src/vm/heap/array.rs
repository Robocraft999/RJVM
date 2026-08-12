use std::slice;
use log::{error, warn};
use crate::vm::jni::types::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort};
use crate::vm::value::{RefId, Value};

pub enum ArrayContent {
    Ref(RawArray<jobject>),
    Bool(RawArray<jboolean>),
    Byte(RawArray<jbyte>),
    Char(RawArray<jchar>),
    Short(RawArray<jshort>),
    Int(RawArray<jint>),
    Long(RawArray<jlong>),
    Float(RawArray<jfloat>),
    Double(RawArray<jdouble>)
}

impl ArrayContent {
    pub fn set(&mut self, index: usize, value: Value) {
        match (self, value) {
            (ArrayContent::Ref(raw), Value::Reference(ref_id)) => raw.set(index, ref_id.nid()),
            (ArrayContent::Bool(raw), Value::Integer(val)) => raw.set(index, val as jboolean),
            (ArrayContent::Byte(raw), Value::Integer(val)) => raw.set(index, val as jbyte),
            (ArrayContent::Char(raw), Value::Integer(val)) => raw.set(index, val as jchar),
            (ArrayContent::Short(raw), Value::Integer(val)) => raw.set(index, val as jshort),
            (ArrayContent::Int(raw), Value::Integer(val)) => raw.set(index, val as jint),
            (ArrayContent::Long(raw), Value::Long(val)) => raw.set(index, val as jlong),
            (ArrayContent::Float(raw), Value::Float(val)) => raw.set(index, val as jfloat),
            (ArrayContent::Double(raw), Value::Double(val)) => raw.set(index, val as jdouble),
            _ => unreachable!("Setting wrong type in array")
        }
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        match self {
            ArrayContent::Ref(raw) => raw.get(index).map(|n| Value::Reference(RefId(*n))),
            ArrayContent::Bool(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            ArrayContent::Byte(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            ArrayContent::Char(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            ArrayContent::Short(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            ArrayContent::Int(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            ArrayContent::Long(raw) => raw.get(index).map(|n| Value::Long(*n as i64)),
            ArrayContent::Float(raw) => raw.get(index).map(|n| Value::Float(*n as f32)),
            ArrayContent::Double(raw) => raw.get(index).map(|n| Value::Double(*n as f64)),
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            ArrayContent::Ref(raw) => raw.len(),
            ArrayContent::Bool(raw) => raw.len(),
            ArrayContent::Byte(raw) => raw.len(),
            ArrayContent::Char(raw) => raw.len(),
            ArrayContent::Short(raw) => raw.len(),
            ArrayContent::Int(raw) => raw.len(),
            ArrayContent::Long(raw) => raw.len(),
            ArrayContent::Float(raw) => raw.len(),
            ArrayContent::Double(raw) => raw.len(),
        }
    }

    pub const fn as_raw_ptr(&self) -> *const u8 {
        match self {
            ArrayContent::Ref(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Bool(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Byte(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Char(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Short(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Int(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Long(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Float(raw) => raw.as_ptr() as *const u8,
            ArrayContent::Double(raw) => raw.as_ptr() as *const u8,
        }
    }

    pub fn get_as_string(&self) -> Option<String> {
        if let ArrayContent::Char(raw) = self {
            Some(String::from_utf16_lossy(unsafe { raw.as_slice() }))
        } else {
            None
        }
    }

    pub fn as_ref_vec(&self) -> Option<Vec<RefId>> {
        if let ArrayContent::Ref(raw) = self {
            Some(unsafe { raw.as_slice() }.into_iter().map(|r| RefId(*r)).collect())
        } else {
            None
        }
    }

    pub fn as_byte_vec(&self) -> Option<Vec<u8>> {
        if let ArrayContent::Byte(raw) = self {
            Some(unsafe { raw.as_slice() }.into_iter().map(|r| *r as u8).collect())
        } else {
            None
        }
    }

    pub fn as_vec(&self) -> Vec<Value> {
        match self {
            ArrayContent::Ref(raw)    => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Reference(RefId(*n))).collect(),
            ArrayContent::Bool(raw)   => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Integer(*n as i32)).collect(),
            ArrayContent::Byte(raw)   => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Integer(*n as i32)).collect(),
            ArrayContent::Char(raw)   => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Integer(*n as i32)).collect(),
            ArrayContent::Short(raw)  => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Integer(*n as i32)).collect(),
            ArrayContent::Int(raw)    => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Integer(*n as i32)).collect(),
            ArrayContent::Long(raw)   => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Long(*n as i64)).collect(),
            ArrayContent::Float(raw)  => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Float(*n as f32)).collect(),
            ArrayContent::Double(raw) => unsafe { raw.as_slice() }.into_iter().map(|n| Value::Double(*n as f64)).collect(),
        }
    }

    pub fn copy(&self, dst: &Self, src_offset: usize, dst_offset: usize, length: usize) {
        match (self, dst) {
            (ArrayContent::Ref(src), ArrayContent::Ref(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Bool(src), ArrayContent::Bool(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Byte(src), ArrayContent::Byte(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Char(src), ArrayContent::Char(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Short(src), ArrayContent::Short(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Int(src), ArrayContent::Int(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Long(src), ArrayContent::Long(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Float(src), ArrayContent::Float(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (ArrayContent::Double(src), ArrayContent::Double(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            _ => unreachable!("Cannot copy between different typed arrays")
        }
    }

    pub const fn is_ref(&self) -> bool { matches!(self, ArrayContent::Ref(..)) }
    pub const fn is_bool(&self) -> bool { matches!(self, ArrayContent::Bool(..)) }
    pub const fn is_byte(&self) -> bool { matches!(self, ArrayContent::Byte(..)) }
    pub const fn is_char(&self) -> bool { matches!(self, ArrayContent::Char(..)) }
    pub const fn is_short(&self) -> bool { matches!(self, ArrayContent::Short(..)) }
    pub const fn is_int(&self) -> bool { matches!(self, ArrayContent::Int(..)) }
    pub const fn is_long(&self) -> bool { matches!(self, ArrayContent::Long(..)) }
    pub const fn is_float(&self) -> bool { matches!(self, ArrayContent::Float(..)) }
    pub const fn is_double(&self) -> bool { matches!(self, ArrayContent::Double(..)) }
}

#[repr(C)]
pub struct RawArray<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> RawArray<T> {
    pub fn new(ptr: *mut T, len: usize) -> Self {
        Self {
            ptr,
            len
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        unsafe { &*self.ptr.add(index) }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            warn!("[RawArray]: Getting out of bounds");
            None
        }
    }

    pub unsafe fn set_unchecked(&self, index: usize, value: T) {
        unsafe { *self.ptr.add(index) = value }
    }

    pub fn set(&self, index: usize, value: T) {
        if index < self.len {
            unsafe { self.set_unchecked(index, value) }
        } else {
            error!("[RawArray]: Setting out of bounds")
        }
    }

    pub unsafe fn unchecked_copy_from(&self, from: *const T, offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if offset + length - 1 < self.len {
            unsafe { self.ptr.add(offset).copy_from(from, length) }
        } else {
            error!("[RawArray]: Array is to small for requested copy from")
        }
    }

    pub fn copy_to(&self, to: &Self, src_offset: usize, dst_offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if src_offset + length - 1 < self.len && dst_offset + length - 1 < to.len {
            unsafe { self.ptr.add(src_offset).copy_to(to.ptr.add(dst_offset), length) }
        } else {
            error!("[RawArray]: Either array is to small for requested copy")
        }
    }

    pub unsafe fn unchecked_copy_to(&self, to: *mut T, offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if offset + length - 1 < self.len {
            unsafe { self.ptr.add(offset).copy_to(to, length) }
        } else {
            error!("[RawArray]: Array is to small for requested copy to")
        }
    }

    pub const fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    pub const fn as_ptr(&self) -> *const T {
        self.ptr
    }
}

unsafe impl<T> Send for RawArray<T> {}
unsafe impl<T> Sync for RawArray<T> {}
