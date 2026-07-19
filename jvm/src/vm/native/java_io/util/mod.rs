use crate::vm::constants::classes::JAVA_IO_FILE_NOT_FOUND_EXCEPTION;
use crate::vm::constants::FILEDESCRIPTOR_fd_INDEX;
use crate::vm::java_thread::JavaThread;
use crate::vm::native::wrap_init;
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, Value};
use crate::vm::Context;
use std::ffi::CString;
use std::str::FromStr;

#[cfg(unix)]
use linux_impl as imp;

#[cfg(windows)]
use windows_impl as imp;

#[cfg(unix)]
pub mod linux_impl;

pub fn get_fd(ctx: Context, this_ref: Reference, fd_field_index: usize) -> VMResult<i32> {
    let fd_val = this_ref.get_ref_field(fd_field_index)?;
    let fd_ref = ctx.vm.resolve_object_by_id(fd_val)?;
    fd_ref.get_int_field(FILEDESCRIPTOR_fd_INDEX)
}

pub fn set_fd(ctx: Context, this_ref: Reference, fd_field_index: usize, fd: i32) -> VMResult<()> {
    let fd_val = this_ref.get_ref_field(fd_field_index)?;
    let fd_ref = ctx.vm.resolve_object_by_id(fd_val)?;
    fd_ref.set_field(FILEDESCRIPTOR_fd_INDEX, Value::Integer(fd));
    Ok(())
}

pub fn file_open(ctx: Context, this_ref: Reference, path_val: Value, fd_field_index: usize, flags: i32) -> VMPartialResult<()> {
    let path = ctx.vm.extract_string_from_value(path_val)?;

    let fd = unsafe { imp::handle_open(CString::from_str(path.as_str()).unwrap(), flags, 0o666) };
    if fd != -1 {
        set_fd(ctx, this_ref, fd_field_index, fd)?;

        Ok(VMResultType::Successful(()))
    } else {
        let exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_FILE_NOT_FOUND_EXCEPTION)?);
        JavaThread::throw(
            ctx,
            exception_clazz,
            path,
            "io: file_open".to_owned()
        )
    }
}

// windows uses handles so use alias
pub use imp::file_close as file_close;

pub use imp::handle_available;

pub fn read_bytes(context: Context, this_ref: Reference, bytes_arr_ref: Reference, offset: i32, length: i32, fd_field_index: usize) -> VMResult<i32> {

    Ok(1)
}