use crate::vm::constants::classes::{JAVA_IO_FILE_INPUT_STREAM, JAVA_IO_FILE_OUTPUT_STREAM, JAVA_IO_UNIX_FILE_SYSTEM};
use crate::vm::constants::{FILEINPUTSTREAM_path_INDEX, FILE_path_INDEX};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{VmError, VM};
use log::{debug, warn};
use std::path::Path;
use std::time::SystemTime;
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_IO_FILE_OUTPUT_STREAM, "writeBytes", "([BIIZ)V", delegate_write_bytes);
    registry.register(JAVA_IO_FILE_INPUT_STREAM, "readBytes", "([BII)I", delegate_read_bytes);
    registry.register(JAVA_IO_FILE_INPUT_STREAM,"open0", "(Ljava/lang/String;)V", delegate_open0);
    registry.register(JAVA_IO_FILE_INPUT_STREAM,"close0", "()V", delegate_close0);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "getBooleanAttributes0", "(Ljava/io/File;)I", delegate_get_boolean_attribute);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "canonicalize0", "(Ljava/lang/String;)Ljava/lang/String;", delegate_canonicalize0);
    registry.register(JAVA_IO_UNIX_FILE_SYSTEM, "getLastModifiedTime", "(Ljava/io/File;)J", delegate_last_modified_time);
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
            let data = &data.borrow()[*offset as usize..(*offset + *amount) as usize];
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
        let io_exception_class = wrap_init!(ctx, ctx.get_or_initialize_class("java/io/IOException")?);

        if let Some(fis_ref) = obj_ref{
            let path = ctx.vm.extract_string_from_value(fis_ref.get_field(FILEINPUTSTREAM_path_INDEX))?;

            let existing_file = ctx.vm.currently_open_files.write()?.remove(&path);
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
                        ctx.vm.currently_open_files.write()?.insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes to end");
                        non_failing_some(Value::Integer((new_index - index) as i32))
                    } else {
                        //read >0 bytes
                        ctx.vm.currently_open_files.write()?.insert(path.clone(), (content, new_index));
                        //println!("read >0 bytes");
                        non_failing_some(Value::Integer((end - start) as i32))
                    }
                } else {
                    if new_index == content.len(){
                        //read 0 bytes from end to end
                        ctx.vm.currently_open_files.write()?.insert(path.clone(), (content, new_index));
                        //println!("read 0 bytes from end to end");
                        non_failing_some(Value::Integer(-1))
                    } else {
                        //read 0 bytes
                        ctx.vm.currently_open_files.write()?.insert(path.clone(), (content, new_index));
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
                    io_exception_class,
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

//obsolete because libjava.so is loaded
gen_delegate!(delegate_open0, |ctx, _obj_ref, args| {
    if let Some(path_val) = args.get(0) && !path_val.is_null(){
        let path = ctx.vm.extract_string_from_value(*path_val)?;
        if !ctx.vm.currently_open_files.read()?.contains_key(&path) {
            let file_content = ctx.vm.class_manager.class_path.resolve_file(path.as_str())?;
            if let Some(file_content) = file_content {
                ctx.vm.currently_open_files.write()?.insert(path.clone(), (file_content, 0));
            }
        }
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
    if ctx.vm.currently_open_files.write()?.remove(&path).is_none() {
        warn!("Closing non existent file: '{}'", path)
    }
    non_failing_none()
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