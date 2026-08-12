use std::ffi::CString;
use crate::vm::constants::classes::{JAVA_IO_FILE_INPUT_STREAM, JAVA_IO_FILE_OUTPUT_STREAM, JAVA_IO_UNIX_FILE_SYSTEM, JAVA_IO_IOEXCEPTION, JAVA_LANG_STRING, JAVA_IO_RANDOM_ACCESS_FILE, JAVA_LANG_STRING_ARR};
use crate::vm::constants::{FILEDESCRIPTOR_fd_INDEX, FILEINPUTSTREAM_fd_INDEX, FILEINPUTSTREAM_path_INDEX, FILEOUTPUTSTREAM_fd_INDEX, FILE_path_INDEX, RANDOMACCESSFILE_fd_INDEX};
use crate::vm::java_thread::JavaThread;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, promote_exception, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{Context, VmError};
use log::{debug, error, warn};
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;
use libc::{c_int, stat64, FIONREAD};
use parking_lot::RwLock;
use crate::class_file::fields::field_type::FieldType;
use crate::vm::heap::array::ArrayContent;

mod util;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_IO_FILE_OUTPUT_STREAM, "writeBytes", "([BIIZ)V", delegate_write_bytes);
    registry.register(JAVA_IO_FILE_INPUT_STREAM, "readBytes", "([BII)I", delegate_read_bytes);
    registry.register(JAVA_IO_FILE_INPUT_STREAM,"open0", "(Ljava/lang/String;)V", delegate_open0);
    registry.register(JAVA_IO_FILE_INPUT_STREAM,"close0", "()V", delegate_close0);
    registry.register(JAVA_IO_FILE_INPUT_STREAM, "available0", "()I", delegate_available0);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "getLastModifiedTime", "(Ljava/io/File;)J", delegate_last_modified_time);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "checkAccess", "(Ljava/io/File;I)Z", delegate_check_access);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "list", "(Ljava/io/File;)[Ljava/lang/String;", delegate_list);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "createDirectory", "(Ljava/io/File;)Z", delegate_create_directory);
    registry.register(JAVA_IO_RANDOM_ACCESS_FILE, "open0", "(Ljava/lang/String;I)V", delegate_raf_open0);
    registry.register(JAVA_IO_RANDOM_ACCESS_FILE, "close0", "()V", delegate_raf_close0);
}

gen_delegate!(delegate_write_bytes, |ctx, obj_ref, args| {
    if let (
        Some(fis_ref),
        Some(Value::Reference(bytes_ref_id)),
        Some(Value::Integer(offset)),
        Some(Value::Integer(amount)),
        Some(Value::Integer(should_append))
    ) = (obj_ref, args.get(0), args.get(1), args.get(2), args.get(3)) {
        let bytes_ref = ctx.vm.resolve_object_by_id(*bytes_ref_id)?;
        if let ReferenceType::Array(data) = &bytes_ref.reference_type {
            if let ArrayContent::Byte(raw) = &*data.read() {
                if offset + amount -1 < raw.len() as i32 {
                    promote_exception!(util::write_bytes(ctx, fis_ref, raw.as_ptr(), *offset, *amount, *should_append, FILEOUTPUTSTREAM_fd_INDEX)?);
                } else {
                    error!(target: "native", "write_bytes: out of range")
                }
                non_failing_none()
            } else {
                invalidation!("Expected a byte array as first arg")
            }
        } else {
            invalidation!("Expected an array as first arg")
        }
    } else {
        invalidation!("Expected a byte array, offset, amount and boolean")
    }
});

gen_delegate!(delegate_read_bytes, |ctx, obj_ref, args| {
    if let (Some(Value::Reference(data_ref_id)), Some(Value::Integer(offset)), Some(Value::Integer(amount))) = (args.get(0), args.get(1), args.get(2)) {
        // let io_exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);

        if let Some(fis_ref) = obj_ref{
            let bytes_ref = ctx.vm.resolve_object_by_id(*data_ref_id)?;
            if let ReferenceType::Array(data) = &bytes_ref.reference_type {
            if let ArrayContent::Byte(raw) = &*data.write() {
                if offset + amount -1 < raw.len() as i32 {
                    let read_bytes = promote_exception!(util::read_bytes(ctx, fis_ref, raw.as_mut_ptr(), *offset, *amount, FILEINPUTSTREAM_fd_INDEX)?);
                        non_failing_some(Value::Integer(read_bytes))
                } else {
                    invalidation!("read_bytes: IndexOutOfBounds")
                }
            } else {
                invalidation!("Expected a byte array as first arg")
            }
        } else {
            invalidation!("Expected an array as first arg")
        }

        } else {
            invalidation!("Expected an object reference")
        }
    } else {
        invalidation!("Expected a byte array, integer and integer as args")
    }
});

//obsolete because libjava.so is loaded
gen_delegate!(delegate_open0, |ctx, obj_ref, args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    if let Some(path_val) = args.get(0) && !path_val.is_null(){
        promote_exception!(util::file_open(ctx, fis_ref, *path_val, FILEINPUTSTREAM_fd_INDEX, libc::O_RDONLY)?);
        non_failing_none()
    } else {
        invalidation!("Expected a string for the path but got: {:?}", args.get(0))
    }
});

gen_delegate!(delegate_close0, |ctx, obj_ref, _args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    let path_val = fis_ref.get_field(FILEINPUTSTREAM_path_INDEX);
    let path = ctx.vm.extract_string_from_value(path_val)?;
    if ctx.vm.currently_open_files.write().remove(&path).is_none() {
        warn!("Closing non existent file: '{}'", path)
    }
    promote_exception!(unsafe { util::file_close(ctx, fis_ref, FILEINPUTSTREAM_fd_INDEX)? });
    non_failing_none()
});


gen_delegate!(delegate_available0, |ctx, obj_ref, _args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    let fd_val = fis_ref.get_ref_field(FILEINPUTSTREAM_fd_INDEX)?;
    let fd_ref = ctx.vm.resolve_object_by_id(fd_val)?;
    // check how much is still available: https://github.com/openjdk/jdk8u/blob/master/jdk/src/share/native/java/io/FileInputStream.c#L93
    let fd = fd_ref.get_int_field(FILEDESCRIPTOR_fd_INDEX)?;
    if fd == -1 {
        let io_exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);
        return JavaThread::throw(
            ctx,
            io_exception_clazz,
            "Stream Closed".to_owned(),
            "FileInputStream.available0()I".to_owned()
        );
    }

    let (res, amt) = unsafe { util::handle_available(fd) };
    if res {
        let ret = if amt > i32::MAX as i64 {
            i32::MAX
        } else if amt < 0 {
            0
        } else {
            amt as i32
        };
        non_failing_some(Value::Integer(ret))
    } else {
        error!("FileInputStream.available0()I had libc io errors");
        non_failing_some(Value::Integer(0))
    }
});

const BA_EXISTS: i32 = 1;
const BA_REGULAR: i32 = 2;
const BA_DIRECTORY: i32 = 4;
const BA_HIDDEN: i32 = 8;

gen_delegate!(delegate_get_boolean_attribute, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(path_ref_id)) = args.get(0){
        let path_ref = ctx.vm.resolve_object_by_id(*path_ref_id)?;
        let string_val = path_ref.get_field(FILE_path_INDEX);
        let path = ctx.vm.extract_string_from_value(string_val)?;
        let path = Path::new(&path);
        let mut attributes = 0;
        if path.exists(){
            attributes |= BA_EXISTS;
            if path.is_dir(){
                attributes |= BA_DIRECTORY;
            }
        }
        debug!(target: "native", "HILFE {:?} ({}), {}", path, attributes, attributes & BA_EXISTS);
        non_failing_some(Value::Integer(attributes))
    } else {
        invalidation!("Expected file as parameter")
    }
});

gen_delegate!(delegate_canonicalize0, |ctx, _obj_ref, args| {
    debug!("canonicalize0");
    if let Some(string_val) = args.get(0){
        let path = ctx.vm.extract_string_from_value(*string_val)?;
        let path = Path::new(&path);
        let path = path.canonicalize().unwrap().into_os_string().into_string().unwrap();
        let new_path = wrap_init!(ctx, ctx.new_string_object(path.as_str())?);
        non_failing_some(Value::Reference(new_path.id))
    } else {
        invalidation!("Can't canonicalize 0 arguments")
    }
});

gen_delegate!(delegate_last_modified_time, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(path_ref_id)) = args.get(0){
        let path_ref = ctx.vm.resolve_object_by_id(*path_ref_id)?;
        let string_val = path_ref.get_field(FILE_path_INDEX);
        let path = ctx.vm.extract_string_from_value(string_val)?;
        let path = Path::new(&path);
        let last_modified = path.metadata().map(|m| m.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64).unwrap_or(0);
        non_failing_some(Value::Long(last_modified))
    } else {
        invalidation!("Expected file as parameter")
    }
});

const ACCESS_READ:    i32 = 0x04;
const ACCESS_WRITE:   i32 = 0x02;
const ACCESS_EXECUTE: i32 = 0x01;

gen_delegate!(delegate_check_access, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(file_ref_id)), Some(Value::Integer(mode))) = (args.get(0), args.get(1)){
        let file_ref = ctx.vm.resolve_object_by_id(*file_ref_id)?;
        let string_val = file_ref.get_field(FILE_path_INDEX);
        let path = ctx.vm.extract_string_from_value(string_val)?;
        let path = Path::new(&path);
        if !path.exists() {
            return non_failing_some(Value::from(false))
        }
        let permissions = path.metadata().unwrap().permissions().mode();
        let res = Value::from(match *mode {
            ACCESS_READ => permissions & 0o400 != 0,
            ACCESS_WRITE => permissions & 0o200 != 0,
            ACCESS_EXECUTE => permissions & 0o100 != 0,
            _ => unreachable!("Invalid file mode: {}", mode)
        });
        non_failing_some(res)
    } else {
        invalidation!("Expected file and mode as parameter")
    }
});

gen_delegate!(delegate_list, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(file_ref_id)) = args.get(0){
        let file_ref = ctx.vm.resolve_object_by_id(*file_ref_id)?;
        let string_val = file_ref.get_field(FILE_path_INDEX);
        let path = ctx.vm.extract_string_from_value(string_val)?;
        let path = Path::new(&path);

        if !path.exists() {
            return non_failing_some(ctx.vm.null())
        }

        let strings = path
            .read_dir()
            .unwrap()
            .filter_map(|e| e.map(|de| de.file_name().into_string().ok()).ok().flatten())
            .map(|s| ctx.try_new_string_object(&s).map(|r| Value::Reference(r.id)))
            .try_collect::<Vec<_>>()?;

        let arr_clazz = ctx.get_or_resolve_class(JAVA_LANG_STRING_ARR)?;
        let arr = ctx.try_new_array(arr_clazz, strings)?;
        non_failing_some(Value::Reference(arr.id))
    } else {
        invalidation!("Expected File Parameter")
    }
});

gen_delegate!(delegate_create_directory, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(file_ref_id)) = args.get(0){
        let file_ref = ctx.vm.resolve_object_by_id(*file_ref_id)?;
        let string_val = file_ref.get_field(FILE_path_INDEX);
        let path = ctx.vm.extract_string_from_value(string_val)?;
        let path = Path::new(&path);

        let worked = std::fs::create_dir(path).is_ok();
        non_failing_some(Value::from(worked))
    } else {
        invalidation!("Expected File Parameter")
    }
});

const RAF_O_RDONLY: i32 = 1;
const RAF_O_RDWR: i32 = 2;
const RAF_O_SYNC: i32 = 4;
const RAF_O_DSYNC: i32 = 4;

gen_delegate!(delegate_raf_open0, |ctx, obj_ref, args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    if let (Some(path_val), Some(Value::Integer(mode))) = (args.get(0), args.get(1)) && !path_val.is_null(){
        let mut flags = 0;
        if mode & RAF_O_RDONLY > 0 {
            flags = libc::O_RDONLY;
        } else if mode & RAF_O_RDWR > 0 {
            flags = libc::O_RDWR | libc::O_CREAT;
            if mode & RAF_O_SYNC > 0 {
                flags |= libc::O_SYNC;
            } else if mode & RAF_O_DSYNC > 0 {
                flags |= libc::O_DSYNC;
            }
        }
        promote_exception!(util::file_open(ctx, fis_ref, *path_val, RANDOMACCESSFILE_fd_INDEX, flags)?);

        non_failing_none()
    } else {
        invalidation!("Expected a string for the path but got: {:?}", args.get(0))
    }
});

gen_delegate!(delegate_raf_close0, |ctx, obj_ref, _args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    promote_exception!(unsafe { util::file_close(ctx, fis_ref, RANDOMACCESSFILE_fd_INDEX)? });
    non_failing_none()
});