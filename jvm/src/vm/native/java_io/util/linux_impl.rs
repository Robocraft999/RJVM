use std::ffi::CString;
use libc::{c_int, stat64, FIONREAD};
use log::error;
use crate::vm::constants::classes::JAVA_IO_IOEXCEPTION;
use crate::vm::constants::FILEDESCRIPTOR_fd_INDEX;
use crate::vm::Context;
use crate::vm::java_thread::JavaThread;
use crate::vm::native::wrap_init;
use crate::vm::result::{VMPartialResult, VMResultType};
use crate::vm::value::{Reference, Value};

pub unsafe fn handle_available(fd: i32) -> (bool, i64) {
    let mut size: i64 = 0;

    let mut buf64: stat64 = std::mem::zeroed();
    let result = libc::fstat64(fd, &mut buf64);
    if result != -1 {
        let mode = buf64.st_mode;
        if mode & libc::S_IFCHR > 0 || mode & libc::S_IFIFO > 0 || mode & libc::S_IFSOCK > 0 {
            let mut n: c_int = 0;
            let res = libc::ioctl(fd, FIONREAD, &mut n);
            if res >= 0 {
                return (true, n as i64);
            }
        } else if mode & libc::S_IFREG > 0 {
            size = buf64.st_size as i64;
        }
    }

    let current = libc::lseek64(fd, 0, libc::SEEK_CUR);
    if current == -1 {
        return (false, 0)
    }

    if size < current {
        size = libc::lseek64(fd, 0, libc::SEEK_END);
        if size == -1 {
            return (false, 0)
        } else if libc::lseek64(fd, current, libc::SEEK_SET) == -1 {
            return (false, 0)
        }
    }

    (true, size - current)
}

pub unsafe fn handle_open(path: CString, oflags: i32, mode: i32) -> i32 {
    let fd = libc::open64(path.as_ptr(), oflags, mode);
    if fd != -1 {
        let mut buf64: stat64 = std::mem::zeroed();
        let result = libc::fstat64(fd, &mut buf64);
        if result != -1 {
            if buf64.st_mode & libc::S_IFDIR > 0 {
                error!("Cannot open a dir");
                libc::close(fd);
                return -1;
            }
        } else {
            libc::close(fd);
            return -1;
        }
    }
    fd as i32
}

pub unsafe fn file_close(ctx: Context, this_ref: Reference, fd_field_index: usize) -> VMPartialResult<()> {
    let fd_val = this_ref.get_ref_field(fd_field_index)?;
    let fd_ref = ctx.vm.resolve_object_by_id(fd_val)?;
    let fd = fd_ref.get_int_field(FILEDESCRIPTOR_fd_INDEX)?;

    if fd == -1 {
        return Ok(VMResultType::Successful(()));
    }

    fd_ref.set_field(FILEDESCRIPTOR_fd_INDEX, Value::Integer(-1));

    let exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);

    if fd >= libc::STDIN_FILENO && fd <= libc::STDERR_FILENO {
        let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
        if devnull < 0 {
            fd_ref.set_field(FILEDESCRIPTOR_fd_INDEX, Value::Integer(fd));
            return JavaThread::throw(
                ctx,
                exception_clazz,
                "open /dev/null failed".to_owned(),
                "io: file_close".to_owned()
            );
        } else {
            libc::dup2(devnull, fd);
            libc::close(fd);
        }
    } else {
        let res = libc::close(fd);
        if res == -1 {
            return JavaThread::throw(
                ctx,
                exception_clazz,
                "close failed".to_owned(),
                "io: file_close".to_owned()
            )
        }
    }

    Ok(VMResultType::Successful(()))
}