use crate::vm::constants::classes::{JAVA_IO_FILE_NOT_FOUND_EXCEPTION, JAVA_IO_IOEXCEPTION};
use crate::vm::constants::FILEDESCRIPTOR_fd_INDEX;
use crate::vm::java_thread::JavaThread;
use crate::vm::jni::types::jbyte;
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

#[cfg(unix)]
use imp::handle_write as io_write;

#[cfg(unix)]
use imp::handle_write as io_append;

#[cfg(unix)]
use imp::handle_read as io_read;

pub fn read_bytes(ctx: Context, this_ref: Reference, bytes: *mut jbyte, offset: i32, length: i32, fd_index: usize) -> VMPartialResult<i32> {
    let length = length as usize;
    let offset = offset as usize;

    if length == 0 {
        return Ok(VMResultType::Successful(0));
    }

    let bytes = unsafe { bytes.add(offset) };

    let io_exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);

    let fd = get_fd(ctx, this_ref, fd_index)?;
    if fd == -1 {
        JavaThread::throw(
            ctx,
            io_exception_clazz,
            "Stream Closed".to_owned(),
            "FileOutputStream.writeBytes([BIIZ)V".to_owned()
        )
    } else {
        let nread = unsafe { io_read(fd, bytes, length) };
        if nread > 0 {
            Ok(VMResultType::Successful(nread as i32))
        } else if nread == -1 {
            JavaThread::throw(
                ctx,
                io_exception_clazz,
                "Read Error".to_owned(),
                "FileOutputStream.writeBytes([BIIZ)V".to_owned()
            )
        } else {
            // EOF
            Ok(VMResultType::Successful(-1))
        }
    }
}

pub fn write_bytes(ctx: Context, this_ref: Reference, bytes: *const jbyte, offset: i32, length: i32, append: i32, fd_index: usize) -> VMPartialResult<()>{
    let length = length as usize;
    let offset = offset as usize;
    let append = append == 1;

    if length == 0 {
        return Ok(VMResultType::Successful(()));
    }

    let bytes = unsafe { bytes.add(offset) };

    let io_exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);

    let fd = get_fd(ctx, this_ref, fd_index)?;

    let mut off = 0;
    let mut len = length;
    let mut n;

    while len > 0 {
        if fd == -1 {
            return JavaThread::throw(
                ctx,
                io_exception_clazz,
                "Stream Closed".to_owned(),
                "FileOutputStream.writeBytes([BIIZ)V".to_owned()
            );
        }

        if append {
            n = unsafe { io_append(fd, bytes.add(off), length) }
        } else {
            n = unsafe { io_write(fd, bytes.add(off), length) }
        }

        if n == -1 {
            return JavaThread::throw(
                ctx,
                io_exception_clazz,
                "Write Error".to_owned(),
                "FileOutputStream.writeBytes([BIIZ)V".to_owned()
            );
        }
        off += n as usize;
        len -= n as usize;
    }


    Ok(VMResultType::Successful(()))
}