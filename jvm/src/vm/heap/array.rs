use crate::vm::jni::types::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort};
use crate::vm::value::{RefId, Value};
use log::{error, warn};
use std::slice;

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
            (Self::Ref(raw), Value::Reference(ref_id)) => raw.set(index, ref_id.nid()),
            (Self::Bool(raw), Value::Integer(val)) => raw.set(index, val as jboolean),
            (Self::Byte(raw), Value::Integer(val)) => raw.set(index, val as jbyte),
            (Self::Char(raw), Value::Integer(val)) => raw.set(index, val as jchar),
            (Self::Short(raw), Value::Integer(val)) => raw.set(index, val as jshort),
            (Self::Int(raw), Value::Integer(val)) => raw.set(index, val as jint),
            (Self::Long(raw), Value::Long(val)) => raw.set(index, val as jlong),
            (Self::Float(raw), Value::Float(val)) => raw.set(index, val as jfloat),
            (Self::Double(raw), Value::Double(val)) => raw.set(index, val as jdouble),
            _ => unreachable!("Setting wrong type in array")
        }
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        match self {
            Self::Ref(raw) => raw.get(index).map(|n| Value::Reference(RefId(*n))),
            Self::Bool(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            Self::Byte(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            Self::Char(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            Self::Short(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            Self::Int(raw) => raw.get(index).map(|n| Value::Integer(*n as i32)),
            Self::Long(raw) => raw.get(index).map(|n| Value::Long(*n as i64)),
            Self::Float(raw) => raw.get(index).map(|n| Value::Float(*n as f32)),
            Self::Double(raw) => raw.get(index).map(|n| Value::Double(*n as f64)),
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Ref(raw) => raw.len(),
            Self::Bool(raw) => raw.len(),
            Self::Byte(raw) => raw.len(),
            Self::Char(raw) => raw.len(),
            Self::Short(raw) => raw.len(),
            Self::Int(raw) => raw.len(),
            Self::Long(raw) => raw.len(),
            Self::Float(raw) => raw.len(),
            Self::Double(raw) => raw.len(),
        }
    }

    pub const fn as_raw_ptr(&self) -> *const u8 {
        match self {
            Self::Ref(raw) => raw.as_ptr().cast::<u8>(),
            Self::Bool(raw) => raw.as_ptr(),
            Self::Byte(raw) => raw.as_ptr().cast::<u8>(),
            Self::Char(raw) => raw.as_ptr().cast::<u8>(),
            Self::Short(raw) => raw.as_ptr().cast::<u8>(),
            Self::Int(raw) => raw.as_ptr().cast::<u8>(),
            Self::Long(raw) => raw.as_ptr().cast::<u8>(),
            Self::Float(raw) => raw.as_ptr().cast::<u8>(),
            Self::Double(raw) => raw.as_ptr().cast::<u8>(),
        }
    }

    pub fn get_as_string(&self) -> Option<String> {
        if let Self::Char(raw) = self {
            Some(String::from_utf16_lossy(unsafe { raw.as_slice() }))
        } else {
            None
        }
    }

    pub fn as_ref_vec(&self) -> Option<Vec<RefId>> {
        if let Self::Ref(raw) = self {
            Some(unsafe { raw.as_slice() }.iter().map(|r| RefId(*r)).collect())
        } else {
            None
        }
    }

    pub fn as_byte_vec(&self) -> Option<Vec<u8>> {
        if let Self::Byte(raw) = self {
            Some(unsafe { raw.as_slice() }.iter().map(|r| *r as u8).collect())
        } else {
            None
        }
    }

    pub fn as_vec(&self) -> Vec<Value> {
        match self {
            Self::Ref(raw)    => unsafe { raw.as_slice() }.iter().map(|n| Value::Reference(RefId(*n))).collect(),
            Self::Bool(raw)   => unsafe { raw.as_slice() }.iter().map(|n| Value::Integer(*n as i32)).collect(),
            Self::Byte(raw)   => unsafe { raw.as_slice() }.iter().map(|n| Value::Integer(*n as i32)).collect(),
            Self::Char(raw)   => unsafe { raw.as_slice() }.iter().map(|n| Value::Integer(*n as i32)).collect(),
            Self::Short(raw)  => unsafe { raw.as_slice() }.iter().map(|n| Value::Integer(*n as i32)).collect(),
            Self::Int(raw)    => unsafe { raw.as_slice() }.iter().map(|n| Value::Integer(*n as i32)).collect(),
            Self::Long(raw)   => unsafe { raw.as_slice() }.iter().map(|n| Value::Long(*n as i64)).collect(),
            Self::Float(raw)  => unsafe { raw.as_slice() }.iter().map(|n| Value::Float(*n as f32)).collect(),
            Self::Double(raw) => unsafe { raw.as_slice() }.iter().map(|n| Value::Double(*n as f64)).collect(),
        }
    }

    pub fn copy(&self, dst: &Self, src_offset: usize, dst_offset: usize, length: usize) {
        match (self, dst) {
            (Self::Ref(src), Self::Ref(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Bool(src), Self::Bool(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Byte(src), Self::Byte(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Char(src), Self::Char(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Short(src), Self::Short(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Int(src), Self::Int(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Long(src), Self::Long(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Float(src), Self::Float(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            (Self::Double(src), Self::Double(dst)) => src.copy_to(dst, src_offset, dst_offset, length),
            _ => unreachable!("Cannot copy between different typed arrays")
        }
    }

    pub const fn is_ref(&self) -> bool { matches!(self, Self::Ref(..)) }
    pub const fn is_bool(&self) -> bool { matches!(self, Self::Bool(..)) }
    pub const fn is_byte(&self) -> bool { matches!(self, Self::Byte(..)) }
    pub const fn is_char(&self) -> bool { matches!(self, Self::Char(..)) }
    pub const fn is_short(&self) -> bool { matches!(self, Self::Short(..)) }
    pub const fn is_int(&self) -> bool { matches!(self, Self::Int(..)) }
    pub const fn is_long(&self) -> bool { matches!(self, Self::Long(..)) }
    pub const fn is_float(&self) -> bool { matches!(self, Self::Float(..)) }
    pub const fn is_double(&self) -> bool { matches!(self, Self::Double(..)) }
}

#[repr(C)]
pub struct RawArray<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> RawArray<T> {
    pub const fn new(ptr: *mut T, len: usize) -> Self {
        Self {
            ptr,
            len
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const unsafe fn as_slice(&self) -> &[T] {
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
            error!("[RawArray]: Setting out of bounds");
        }
    }

    pub unsafe fn unchecked_copy_from(&self, from: *const T, offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if offset + length - 1 < self.len {
            unsafe { self.ptr.add(offset).copy_from(from, length) }
        } else {
            error!("[RawArray]: Array is to small for requested copy from");
        }
    }

    pub fn copy_to(&self, to: &Self, src_offset: usize, dst_offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if src_offset + length - 1 < self.len && dst_offset + length - 1 < to.len {
            unsafe { self.ptr.add(src_offset).copy_to(to.ptr.add(dst_offset), length) }
        } else {
            error!("[RawArray]: Either array is to small for requested copy");
        }
    }

    pub unsafe fn unchecked_copy_to(&self, to: *mut T, offset: usize, length: usize) {
        if length == 0 {
            return;
        }
        if offset + length - 1 < self.len {
            unsafe { self.ptr.add(offset).copy_to(to, length) }
        } else {
            error!("[RawArray]: Array is to small for requested copy to");
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
