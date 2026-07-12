use std::ffi::CString;
use crate::vm::constants::classes::{JAVA_IO_FILE_INPUT_STREAM, JAVA_IO_FILE_OUTPUT_STREAM, JAVA_IO_UNIX_FILE_SYSTEM, JAVA_IO_IOEXCEPTION, JAVA_LANG_STRING, JAVA_IO_RANDOM_ACCESS_FILE};
use crate::vm::constants::{FILEDESCRIPTOR_fd_INDEX, FILEINPUTSTREAM_fd_INDEX, FILEINPUTSTREAM_path_INDEX, FILE_path_INDEX};
use crate::vm::java_thread::JavaThread;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult};
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
    registry.register(JAVA_IO_RANDOM_ACCESS_FILE, "open0", "(Ljava/lang/String;I)V", delegate_raf_open0)
}

gen_delegate!(delegate_write_bytes, |ctx, _obj_ref, args| {
    if let (
        Some(Value::Reference(bytes_ref_id)),
        Some(Value::Integer(offset)),
        Some(Value::Integer(amount)),
        Some(Value::Integer(_should_append))
    ) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let bytes_ref = ctx.vm.resolve_object_by_id(*bytes_ref_id)?;
        if let ReferenceType::Array(_, _, data) = &bytes_ref.reference_type{
            let data = &data.read()[*offset as usize..(*offset + *amount) as usize];
            let string: String = data.iter().map(|value| if let Value::Integer(int) = value { (*int as u8) as char} else { '?' }).collect();
            print!("{}", string);
            non_failing_none()
        } else {
            invalidation!("Expected a byte array as first arg")
        }
    } else {
        invalidation!("Expected a byte array, offset, amount and boolean")
    }
});

gen_delegate!(delegate_read_bytes, |ctx, obj_ref, args| {
    if let (Some(Value::Reference(data_ref_id)), Some(Value::Integer(offset)), Some(Value::Integer(length))) = (args.get(0), args.get(1), args.get(2)) {
        let io_exception_clazz = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_IO_IOEXCEPTION)?);

        if let Some(fis_ref) = obj_ref{
            let path = ctx.vm.extract_string_from_value(fis_ref.get_field(FILEINPUTSTREAM_path_INDEX))?;

            let existing_file = ctx.vm.currently_open_files.write().remove(&path);
            if let Some((content, index)) = existing_file {
                let data_ref = ctx.vm.resolve_object_by_id(*data_ref_id)?;
                //file: len 20, i 5
                //buffer: blen 30, o 10, length 20
                //start = 10, end = 25 = 10 + min(30 - 10, 20 - 5)

                let start = *offset as usize;
                let end = start + std::cmp::min(*length as usize, content.len() - index);
                //println!("XXX: {}", &path);
                //println!("start={}, end={}, readable_bytes={}, reading={:X?}", start, end, content.len() - index, &content[index..(index+end-start)]);
                (start..end).for_each(|i| data_ref.set_element(i, Value::Integer(content[i - start + index] as i32)));

                let new_index = index + end - start;
                if new_index > index{
                    if new_index == content.len(){
                        //read >0 bytes to end
                        ctx.vm.currently_open_files.write().insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes to end");
                        non_failing_some(Value::Integer((new_index - index) as i32))
                    } else {
                        //read >0 bytes
                        ctx.vm.currently_open_files.write().insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes");
                        non_failing_some(Value::Integer((end - start) as i32))
                    }
                } else {
                    if new_index == content.len(){
                        //read 0 bytes from end to end
                        ctx.vm.currently_open_files.write().insert(path.clone(), (content, new_index));
                        //println!("read 0 bytes from end to end");
                        non_failing_some(Value::Integer(-1))
                    } else {
                        //read 0 bytes
                        ctx.vm.currently_open_files.write().insert(path.clone(), (content, new_index));
                        //println!("read 0 bytes");
                        non_failing_some(Value::Integer(0))
                    }
                }

                //println!("{:?}", &content[start..end]);
                /*if *index == content.len()-1{
                    vm.currently_open_files.remove(&path);
                    Ok(Some(Value::Integer(-1)))
                } else {
                    *index += end - start;
                    Ok(Some(Value::Integer((end - start) as i32)))
                }*/
            } else {
                JavaThread::throw(
                    ctx,
                    io_exception_clazz,
                    format!("File {} was not found", path),
                    String::from("java/io/FileInputStream.readBytes([BII)I")
                )
            }
        } else {
            invalidation!("Expected an object reference")
        }
    } else {
        invalidation!("Expected a byte array, integer and integer as args")
    }
});

unsafe fn handle_open(path: CString, oflags: i32, mode: i32) -> i32 {
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

//obsolete because libjava.so is loaded
gen_delegate!(delegate_open0, |ctx, obj_ref, args| {
    let Some(fis_ref) = obj_ref else {
        return invalidation!("Expected this")
    };
    if let Some(path_val) = args.get(0) && !path_val.is_null(){
        let path = ctx.vm.extract_string_from_value(*path_val)?;
        if !ctx.vm.currently_open_files.read().contains_key(&path) {
            let file_content = ctx.vm.class_manager.class_path.resolve_file(path.as_str())?;
            if let Some(file_content) = file_content {
                ctx.vm.currently_open_files.write().insert(path.clone(), (file_content, 0));
            }
        }
        let fd = unsafe { handle_open(CString::from_str(path.as_str()).unwrap(), libc::O_RDONLY, 0o666) };
        let fd_val = fis_ref.get_ref_field(FILEINPUTSTREAM_fd_INDEX)?;
        let fd_ref = ctx.vm.resolve_object_by_id(fd_val)?;
        fd_ref.set_field(FILEDESCRIPTOR_fd_INDEX, Value::Integer(fd));

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
    non_failing_none()
});

unsafe fn handle_available(fd: i32) -> (bool, i64) {
    let mut size: i64 = 0;
    let mut current: i64 = 0;

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

    current = libc::lseek64(fd, 0, libc::SEEK_CUR);
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

    let (res, amt) = unsafe { handle_available(fd) };
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
        println!("HILFE {:?} ({}), {}", path, attributes, attributes & BA_EXISTS);
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
        let new_path = wrap_init!(ctx, ctx.vm.new_string_object(path.as_str())?);
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
            .map(|s| ctx.vm.try_new_string_object(&s).map(|r| Value::Reference(r.id)))
            .try_collect::<Vec<_>>()?;

        let arr = ctx.vm.try_new_array(1, FieldType::Object(JAVA_LANG_STRING.to_owned()).to_array_field_type(1), RwLock::new(strings))?;
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

gen_delegate!(delegate_raf_open0, |ctx, _obj_ref, args| {

});