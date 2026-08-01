use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::vm::class::Class;
use crate::vm::class_manager::{AnonClassInfo, ClassLoadingState};
use crate::vm::constants::classes::{JAVA_LANG_CLASS, SUN_MISC_UNSAFE};
use crate::vm::constants::{FIELD_clazz_INDEX, FIELD_name_INDEX};
use crate::vm::java_thread::ThreadState;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::VmError;
use log::{debug, trace};
use std::thread::{park, park_timeout};
use std::time::{Duration, SystemTime};
use crate::vm::debug::validation::FieldTypeExt;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate| registry.register(SUN_MISC_UNSAFE, method_name, sig, delegate);
    register("arrayBaseOffset", "(Ljava/lang/Class;)I", delegate_array_base_offset);
    register("arrayIndexScale", "(Ljava/lang/Class;)I", delegate_array_index_scale);
    register("addressSize", "()I", delegate_address_size);
    register("objectFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_object_field_offset);
    register("staticFieldOffset", "(Ljava/lang/reflect/Field;)J", delegate_static_field_offset);
    register("putObjectVolatile", "(Ljava/lang/Object;JLjava/lang/Object;)V", delegate_put_object_volatile);
    register("getObjectVolatile", "(Ljava/lang/Object;J)Ljava/lang/Object;", delegate_get_object_volatile);
    register("putIntVolatile", "(Ljava/lang/Object;JI)V", delegate_put_int_volatile);
    register("getIntVolatile", "(Ljava/lang/Object;J)I", delegate_get_int_volatile);
    register("putLongVolatile", "(Ljava/lang/Object;JJ)V", delegate_put_long_volatile);
    register("getLongVolatile", "(Ljava/lang/Object;J)J", delegate_get_long_volatile);
    register("staticFieldBase", "(Ljava/lang/reflect/Field;)Ljava/lang/Object;", delegate_static_field_base);
    register("compareAndSwapObject", "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z", delegate_compare_and_swap_object);
    register("compareAndSwapInt", "(Ljava/lang/Object;JII)Z", delegate_compare_and_swap_int);
    register("compareAndSwapLong", "(Ljava/lang/Object;JJJ)Z", delegate_compare_and_swap_long);
    register("allocateMemory", "(J)J", delegate_allocate_memory);
    register("setMemory", "(Ljava/lang/Object;JJB)V", delegate_set_memory);
    register("copyMemory", "(Ljava/lang/Object;JLjava/lang/Object;JJ)V", delegate_copy_memory);
    register("freeMemory", "(J)V", delegate_free_memory);
    register("pageSize", "()I", delegate_page_size);
    register("putLong", "(JJ)V", delegate_put_long);
    register("getLong", "(J)J", delegate_get_long);
    register("putInt", "(JI)V", delegate_put_int);
    register("getInt", "(J)I", delegate_get_int);
    register("getByte", "(J)B", delegate_get_byte);
    register("putObject", "(Ljava/lang/Object;JLjava/lang/Object;)V", delegate_put_object_volatile);
    register("getObject", "(Ljava/lang/Object;J)Ljava/lang/Object;", delegate_get_object_volatile);
    register("putInt", "(Ljava/lang/Object;JI)V", delegate_put_int_volatile);
    register("getInt", "(Ljava/lang/Object;J)I", delegate_get_int_volatile);
    register("putLong", "(Ljava/lang/Object;JJ)V", delegate_put_long_volatile);
    register("getLong", "(Ljava/lang/Object;J)J", delegate_get_long_volatile);
    register("putOrderedObject", "(Ljava/lang/Object;JLjava/lang/Object;)V", delegate_put_ordered_object);
    register("defineClass", "(Ljava/lang/String;[BIILjava/lang/ClassLoader;Ljava/security/ProtectionDomain;)Ljava/lang/Class;", delegate_define_class);
    register("defineAnonymousClass", "(Ljava/lang/Class;[B[Ljava/lang/Object;)Ljava/lang/Class;", delegate_define_anon_class);
    register("allocateInstance", "(Ljava/lang/Class;)Ljava/lang/Object;", delegate_allocate_instance);
    register("shouldBeInitialized", "(Ljava/lang/Class;)Z", delegate_should_be_initialized);
    register("ensureClassInitialized", "(Ljava/lang/Class;)V", delegate_ensure_initialized);
    register("park", "(ZJ)V", delegate_park);
    register("unpark", "(Ljava/lang/Object;)V", delegate_unpark);
}


const ARRAY_BASE_OFFSET: usize = 16;

gen_delegate!(delegate_array_base_offset, |_ctx, _obj_ref, args| {
    if let Some(Value::Reference(_class_ref)) = args.get(0){
        non_failing_some(Value::Integer(ARRAY_BASE_OFFSET as i32))
    } else {
        invalidation!("Expected a class object reference")
    }
});

gen_delegate!(delegate_array_index_scale, |_ctx, _obj_ref, args| {
    if let Some(Value::Reference(_class_ref)) = args.get(0){
        non_failing_some(Value::Integer(1))
    } else {
        invalidation!("Expected a class object reference")
    }
});

gen_delegate!(delegate_address_size, |_ctx, _obj_ref, _args| {
    non_failing_some(Value::Integer(8))
});

gen_delegate!(delegate_object_field_offset, |ctx, _obj_ref, args| {
    //FIXME calc real offset
    debug!("delegate_object_field_offset: '{:?}'", args);
    if let Some(Value::Reference(field_id)) = args.get(0){
        let field_ref = ctx.vm.resolve_object_by_id(*field_id)?;
        let Value::Reference(class_ref_id) = field_ref.get_field(FIELD_clazz_INDEX) else { return invalidation!("Expected a reference") };
        let clazz = ctx.resolve_clazz_by_class_ref_id(class_ref_id)?;
        let name_val = field_ref.get_field(FIELD_name_INDEX);
        let name = ctx.vm.extract_string_from_value(name_val)?;
        if let Some((index, _)) = clazz.find_field(name.as_str()){
            non_failing_some(Value::Long(index as i64))
        } else {
            invalidation!("Field with name: '{}' does not exist", name)
        }
    } else {
        invalidation!("Expected an Object field reference")
    }
});

gen_delegate!(delegate_static_field_offset, |ctx, obj_ref, args| {
    //non_failing_some(Value::Long(0))
    //TODO check if needed
    delegate_object_field_offset(ctx, obj_ref, args)
});

gen_delegate!(delegate_put_object_volatile, |ctx, _obj_ref, args| {
    debug!("put_object_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index)), Some(Value::Reference(x_id))) = (args.get(0), args.get(1), args.get(3)){
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        // FIXME verify if null or correct field type
        if o.is_array(){
            o.set_element(*index as usize - ARRAY_BASE_OFFSET, Value::Reference(*x_id));
        } else {
            #[cfg(feature = "validation")]
            {
                let clazz = ctx.vm.find_class_by_id(o.class_id).unwrap();
                clazz.field_at_index(*index as usize).unwrap().field_type.validate(Value::Reference(*x_id), ctx)?;
            }
            o.set_field(*index as usize, Value::Reference(*x_id));
        }
        non_failing_none()
    } else {
        invalidation!("Expected an Reference, Long and Reference but got: {:?}", args)
    }
});

gen_delegate!(delegate_get_object_volatile, |ctx, _obj_ref, args| {
    debug!("get_object_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize - ARRAY_BASE_OFFSET));
        }
        let field_value = if o.class_name == JAVA_LANG_CLASS {
            let class_ref = ctx.extract_class_from_class_object(o)?;
            let static_object = ctx.vm.get_static_class_object(class_ref.id).unwrap();
            static_object.get_field(*index as usize)
        } else {
            o.get_field(*index as usize)
        };
        #[cfg(feature = "validation")]
        {
            let clazz = if o.class_name == JAVA_LANG_CLASS {
                ctx.extract_class_from_class_object(o)?
            } else {
                ctx.vm.find_class_by_id(o.class_id).unwrap()
            };
            clazz.field_at_index(*index as usize).unwrap().field_type.validate(field_value, ctx)?;
        }
        non_failing_some(field_value)
    } else {
        invalidation!("Expected an Reference or Array but got: {:?}", args)
    }
});

gen_delegate!(delegate_put_int_volatile, |ctx, _obj_ref, args| {
    debug!("put_int_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index)), Some(Value::Integer(val))) = (args.get(0), args.get(1), args.get(3)){
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        // FIXME verify if null or correct field type
        if o.is_array(){
            o.set_element(*index as usize - ARRAY_BASE_OFFSET, Value::Integer(*val));
        } else {
            o.set_field(*index as usize, Value::Integer(*val));
        }
        non_failing_none()
    } else {
        invalidation!("Expected an Reference, Long and Int but got: {:?}", args)
    }
});

gen_delegate!(delegate_get_int_volatile, |ctx, _obj_ref, args| {
    debug!("get_int_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize - ARRAY_BASE_OFFSET));
        }
        let field_value = if o.class_name == JAVA_LANG_CLASS {
            let class_ref = ctx.extract_class_from_class_object(o)?;
            let static_object = ctx.vm.get_static_class_object(class_ref.id).unwrap();
            static_object.get_field(*index as usize)
        } else {
            o.get_field(*index as usize)
        };
        non_failing_some(field_value)
    } else {
        invalidation!("Expected an Reference or Array but got: {:?}", args)
    }
});

gen_delegate!(delegate_put_long_volatile, |ctx, _obj_ref, args| {
    debug!("put_int_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index)), Some(Value::Long(val))) = (args.get(0), args.get(1), args.get(3)){
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        // FIXME verify if null or correct field type
        if o.is_array(){
            o.set_element(*index as usize - ARRAY_BASE_OFFSET, Value::Long(*val));
        } else {
            o.set_field(*index as usize, Value::Long(*val));
        }
        non_failing_none()
    } else {
        invalidation!("Expected an Reference, Long and Long but got: {:?}", args)
    }
});

gen_delegate!(delegate_get_long_volatile, |ctx, _obj_ref, args| {
    debug!("get_long_volatile args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index))) = (args.get(0), args.get(1)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if o.is_array(){
            return non_failing_some(o.get_element(*index as usize - ARRAY_BASE_OFFSET));
        }
        let field_value = if o.class_name == JAVA_LANG_CLASS {
            let class_ref = ctx.extract_class_from_class_object(o)?;
            let static_object = ctx.vm.get_static_class_object(class_ref.id).unwrap();
            static_object.get_field(*index as usize)
        } else {
            o.get_field(*index as usize)
        };
        non_failing_some(field_value)
    } else {
        invalidation!("Expected an Reference or Array but got: {:?}", args)
    }
});

gen_delegate!(delegate_static_field_base, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(field_ref_id)) = args.get(0){
        let field_object = ctx.vm.resolve_object_by_id(*field_ref_id)?;
        trace!("staticFieldBase: on field: '{:?}'", field_object);
        let class_object = field_object.get_field(FIELD_clazz_INDEX);
        non_failing_some(class_object)
    } else {
        invalidation!("Expected a field reference")
    }
});

gen_delegate!(delegate_compare_and_swap_object, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(o_id)), Some(Value::Long(offset)), Some(Value::Reference(expected_id)), Some(Value::Reference(x))) = (args.get(0), args.get(1), args.get(3), args.get(4)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if o.is_null(){
            return invalidation!("Expected an object or array but found null")
        } else if o.is_object(){
            if let Value::Reference(current_id) = o.get_field(*offset as usize){
                if current_id == *expected_id{
                    #[cfg(feature = "validation")]
                    {
                        let clazz = ctx.vm.find_class_by_id(o.class_id).unwrap();
                        clazz.field_at_index(*offset as usize).unwrap().field_type.validate(Value::Reference(*x), ctx)?;
                    }
                    o.set_field(*offset as usize, Value::Reference(*x));
                    return non_failing_some(Value::from(true));
                }
            }
        } else if o.is_array(){
            if let Value::Reference(current_id) = o.get_element(*offset as usize - ARRAY_BASE_OFFSET){
                if current_id == *expected_id{
                    o.set_element(*offset as usize - ARRAY_BASE_OFFSET, Value::Reference(*x));
                    return non_failing_some(Value::from(true));
                }
            }
        }
    }
    non_failing_some(Value::from(false))
});

gen_delegate!(delegate_compare_and_swap_int, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(o_id)), Some(Value::Long(offset)), Some(Value::Integer(expected)), Some(Value::Integer(x))) = (args.get(0), args.get(1), args.get(3), args.get(4)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if let Value::Integer(current) = o.get_field(*offset as usize){
            if current == *expected{
                o.set_field(*offset as usize, Value::Integer(*x));
                return non_failing_some(Value::from(true));
            }
        }
    }
    non_failing_some(Value::from(false))
});

gen_delegate!(delegate_compare_and_swap_long, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(o_id)), Some(Value::Long(offset)), Some(Value::Long(expected)), Some(Value::Long(x))) = (args.get(0), args.get(1), args.get(3), args.get(5)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if let Value::Long(current) = o.get_field(*offset as usize){
            if current == *expected{
                o.set_field(*offset as usize, Value::Long(*x));
                return non_failing_some(Value::from(true));
            }
        }
    }
    non_failing_some(Value::from(false))
});

gen_delegate!(delegate_allocate_memory, |ctx, _obj_ref, args| {
    if let Some(Value::Long(num)) = args.get(0){
        //return is address in memory
        let ptr = ctx.vm.unsafe_allocator.allocate_memory(*num as usize);
        non_failing_some(Value::Long(ptr))
    } else {
        invalidation!("Expected a long")
    }
});

gen_delegate!(delegate_set_memory, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(o_id)), Some(Value::Long(offset)), Some(Value::Long(bytes)), Some(Value::Integer(value))) = (args.get(0), args.get(1), args.get(3), args.get(5)){
        if o_id.is_null() {
            ctx.vm.unsafe_allocator.set_memory(*offset, *bytes as usize, *value as u8)?;
            non_failing_none()
        } else {
            unimplemented!("Idk how to set mem on an object")
        }
    } else {
        invalidation!("Expected reference, two longs and a byte")
    }
});

gen_delegate!(delegate_copy_memory, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(o_id1)), Some(Value::Long(offset1)), Some(Value::Reference(o_id2)), Some(Value::Long(offset2)), Some(Value::Long(length))) = (args.get(0), args.get(1), args.get(3), args.get(4), args.get(6)){
        if o_id1.is_null() && o_id2.is_null() {
            ctx.vm.unsafe_allocator.copy_memory(*offset1, *offset2, *length as usize)?;
            non_failing_none()
        } else {
            let None = ctx.vm.resolve_object_by_id(*o_id1).ok() else { unimplemented!("copy memory does not support object src yet") };
            let o_ref2 = ctx.vm.resolve_object_by_id(*o_id2).ok();
            if let Some(o_ref2) = o_ref2 && o_ref2.is_array() {
                /*let mode = if offset1 % 8 == 0 && offset2 % 8 == 0 && length % 8 == 0 {
                    8
                } else if offset1 % 4 == 0 && offset2 % 4 == 0 && length % 4 == 0 {
                    4
                } else {
                    2
                };
                let bytes = ctx.vm.unsafe_allocator.get_bytes(*offset1, *length as usize)?;
                let vals: Vec<Value> = match mode {
                    8 => bytes.into_chunks::<8>().into_iter().map(|b| Value::Long(i64::from_le_bytes(b))).collect(),
                    4 => bytes.into_chunks::<4>().into_iter().map(|b| Value::Integer(i32::from_le_bytes(b))).collect(),
                    2 => bytes.into_chunks::<2>().into_iter().map(|b| Value::Integer(i16::from_le_bytes(b) as i32)).collect(),
                    _ => unreachable!()
                };
                let start = *offset2 as usize - ARRAY_BASE_OFFSET;
                let end = start + (*length as usize) / mode;
                for i in start..end {
                    o_ref2.set_element(i, vals[i])
                }*/
                if o_ref2.class_name == "[B" {
                    let bytes = ctx.vm.unsafe_allocator.get_bytes(*offset1, *length as usize)?;
                    let start = *offset2 as usize - ARRAY_BASE_OFFSET;
                    let end = start + (*length as usize);
                    for i in start..end {
                        o_ref2.set_element(i, Value::Integer(bytes[i] as i32));
                    }
                    non_failing_none()
                } else {
                    unimplemented!("copying to other than byte array is unsupported for now")
                }
            } else {
                invalidation!("dest has to be an array")
            }
        }
    } else {
        invalidation!("Expected reference, two longs and a byte")
    }
});

gen_delegate!(delegate_free_memory, |ctx, _obj_ref, args| {
    if let Some(Value::Long(num)) = args.get(0){
        ctx.vm.unsafe_allocator.free_memory(*num);
        non_failing_none()
    } else {
        invalidation!("Expected a long")
    }
});

const PAGE_SIZE: i32 = 4096;
gen_delegate!(delegate_page_size, |_ctx, _obj_ref, _args| {
    non_failing_some(Value::Integer(PAGE_SIZE))
});

gen_delegate!(delegate_put_long, |ctx, _obj_ref, args| {
    //because args = [Long, Dummy, Long, Dummy]
    if let (Some(Value::Long(ptr)), Some(Value::Long(value))) = (args.get(0), args.get(2)){
        ctx.vm.unsafe_allocator.put_long(*ptr, *value)?;
        non_failing_none()
    } else {
        invalidation!("Expected a long as address and a long as value")
    }
});

gen_delegate!(delegate_get_long, |ctx, _obj_ref, args| {
    if let Some(Value::Long(ptr)) = args.get(0){
        let val = ctx.vm.unsafe_allocator.get_long(*ptr)?;
        non_failing_some(Value::Long(val))
    } else {
        invalidation!("Expected a long as address")
    }
});

gen_delegate!(delegate_put_int, |ctx, _obj_ref, args| {
    //because args = [Long, Dummy, Int]
    if let (Some(Value::Long(ptr)), Some(Value::Integer(value))) = (args.get(0), args.get(2)){
        ctx.vm.unsafe_allocator.put_int(*ptr, *value)?;
        non_failing_none()
    } else {
        invalidation!("Expected a long as address and a int as value")
    }
});

gen_delegate!(delegate_get_int, |ctx, _obj_ref, args| {
    if let Some(Value::Long(ptr)) = args.get(0){
        let val = ctx.vm.unsafe_allocator.get_int(*ptr)?;
        non_failing_some(Value::Integer(val))
    } else {
        invalidation!("Expected a long as address")
    }
});

gen_delegate!(delegate_get_byte, |ctx, _obj_ref, args| {
    if let Some(Value::Long(ptr)) = args.get(0){
        let val = ctx.vm.unsafe_allocator.get_byte(*ptr)?;
        non_failing_some(Value::Integer(val as i32))
    } else {
        invalidation!("Expected a long as address")
    }
});

gen_delegate!(delegate_put_ordered_object, |ctx, _obj_ref, args| {
    debug!("put_ordered_object args: {:?}", args);
    if let (Some(Value::Reference(o_id)), Some(Value::Long(index)), Some(x)) = (args.get(0), args.get(1), args.get(3)) {
        let o = ctx.vm.resolve_object_by_id(*o_id)?;
        if o.is_array(){
            o.set_element(*index as usize - ARRAY_BASE_OFFSET, x.clone());
            return non_failing_none();
        }
        if o.class_name == JAVA_LANG_CLASS {
            let class_ref = ctx.extract_class_from_class_object(o)?;
            let _ = wrap_init!(ctx, ctx.ensure_initialized(class_ref)?);
            let static_object = ctx.vm.static_class_objects.read().get(&class_ref.id).unwrap().clone();
            #[cfg(feature = "validation")]
            {
                class_ref.field_at_index(*index as usize).unwrap().field_type.validate(x.clone(), ctx)?;
            }
            static_object.set_field(*index as usize, x.clone());
        } else {
            #[cfg(feature = "validation")]
            {
                let class_ref = ctx.vm.find_class_by_id(o.class_id).unwrap();
                class_ref.field_at_index(*index as usize).unwrap().field_type.validate(x.clone(), ctx)?;
            }
            o.set_field(*index as usize, x.clone());
        }
        non_failing_none()
    } else {
        invalidation!("Expected a reference or array but got: {:?}", args)
    }
});

gen_delegate!(delegate_define_class, |ctx, _obj_ref, args| {
    if let (Some(class_name_value), Some(Value::Reference(bytes_ref_id)), Some(Value::Integer(start)), Some(Value::Integer(end))) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
        let class_name = ctx.vm.extract_string_from_value(*class_name_value)?;
        let bytes_ref = ctx.vm.resolve_object_by_id(*bytes_ref_id)?;
        let bytes = if let ReferenceType::Array(_, _, data) = &bytes_ref.reference_type{
            data.read().iter().map(|val| if let Value::Integer(byte) = val {*byte as u8} else {0}).collect()
        } else {
            Vec::new()
        };
        let bytes = bytes.into_iter().skip(*start as usize).take((*end - *start) as usize).collect::<Vec<_>>();
        let class_object = wrap_init!(ctx, ctx.define_class(class_name.as_str(), bytes.clone())?);
        non_failing_some(Value::Reference(class_object.id))
    } else {
        invalidation!("define_class: expected string_object, byte array, start and end ints but got: {:?}, {:?}, {:?}, {:?}", args.get(0), args.get(1), args.get(2), args.get(3))
    }
});

gen_delegate!(delegate_define_anon_class, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(host_class_id)), Some(Value::Reference(byte_arr_ref_id)), Some(Value::Reference(_cp_patch_arr_ref))) = (args.get(0), args.get(1), args.get(2)) {
        let host_class_ref = ctx.vm.resolve_object_by_id(*host_class_id)?;
        let byte_arr_ref = ctx.vm.resolve_object_by_id(*byte_arr_ref_id)?;
        if let ReferenceType::Array(_, _, bytes ) = &byte_arr_ref.reference_type{
            let bytes = bytes.read().iter().map(|val| if let Value::Integer(byte) = val {*byte as u8} else {0}).collect::<Vec<_>>();
            let class = ctx.vm.class_manager.define_class(&ctx, None, None, bytes)?;
            
            let class_ref = match ctx.vm.class_manager.classes.lock() {
                Ok(class_lock) => {
                    let class_ref = class_lock.alloc(class);
                    unsafe {
                        let class_ptr: *const Class<'a> = class_ref;
                        &*class_ptr
                    }
                }
                Err(e) => return Err(VmError::from(e))
            };
            
            // we have to assign 'this' here, because we can't resolve it later by name
            class_ref.constants.write()?[class_ref.this_index as usize - 1] = ConstantPoolEntry::Class(class_ref);

            ctx.vm.class_manager.classes_by_id.write()?.insert(class_ref.id, class_ref);
            //vm.class_manager.classes_by_name.borrow_mut().insert(class_ref.name.clone(), class_ref);
            ctx.vm.class_manager.class_loading_states.write()?.insert(class_ref.id, ClassLoadingState::LOADED);
            let class_obj = wrap_init!(ctx, ctx.new_class_object_by_class(class_ref)?);
            ctx.vm.class_manager.anonymous_classes.write()?.insert(class_obj.id, AnonClassInfo { clazz: class_ref, host: host_class_ref });
            non_failing_some(Value::Reference(class_obj.id))
        } else {
            invalidation!("define_anon_class: expected bytes array type but got: {:?}", byte_arr_ref)
        }
    } else {
        invalidation!("define_anon_class: expected three objects, got {:?}", args)
    }
});

gen_delegate!(delegate_allocate_instance, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(class_ref_id)) = args.get(0){
        let clazz = ctx.resolve_clazz_by_class_ref_id(*class_ref_id)?;
        wrap_init!(ctx, ctx.ensure_initialized(clazz)?);
        let object = ctx.new_object_from_class(clazz);
        non_failing_some(Value::Reference(object.id))
    } else {
        invalidation!("Expected a class reference to allocate but got: {:?}", args)
    }
});

gen_delegate!(delegate_should_be_initialized, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(class_ref_id)) = args.get(0){
        let clazz = ctx.resolve_clazz_by_class_ref_id(*class_ref_id)?;
        let initialized = ctx.vm.class_manager.expect_class_state(clazz.id, ClassLoadingState::INITIALIZED);
        non_failing_some(Value::from(!initialized))
    } else {
        invalidation!("Expected a class reference but got: {:?}", args)
    }
});

gen_delegate!(delegate_ensure_initialized, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(class_ref_id)) = args.get(0){
        let clazz = ctx.resolve_clazz_by_class_ref_id(*class_ref_id)?;
        let _clazz = wrap_init!(ctx, ctx.ensure_initialized(clazz)?);
        non_failing_none()
    } else {
        invalidation!("Expected a class reference but got: {:?}", args)
    }
});

gen_delegate!(delegate_park, |ctx, _obj_ref, args| {
    let (Some(Value::Integer(is_absolute)), Some(Value::Long(time))) = (args.get(0), args.get(1)) else { return invalidation!("Expected boolean and long parameters") };

    {
        let mut unparked_flag_lock = ctx.thread.meta.unsafe_unpark_count.lock();
        if *unparked_flag_lock > 0 {
            *unparked_flag_lock -= 1;
            return non_failing_none();
        }
    }

    if *time > 0 {
        if *is_absolute == 0 {
            park_timeout(Duration::from_millis(*time as u64));
        } else if *is_absolute == 1 {
            let amount = Duration::from_millis(*time as u64) - SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            park_timeout(amount);
        } else {
            unreachable!("Boolean cannot be {}", is_absolute);
        }
    } else {
        ctx.thread.meta.park();
        park();
    }
    ctx.thread.meta.unpark();
    non_failing_none()
});

gen_delegate!(delegate_unpark, |ctx, _obj_ref, args| {
    let Some(Value::Reference(thread_ref_id)) = args.get(0) else { return invalidation!("Expected Thread ref") };
    let Some(meta) = ctx.vm.thread_lookup.read().get(thread_ref_id).cloned() else {
        return invalidation!("Reference with {:?} has no associated JavaThread", thread_ref_id)
    };

    if *meta.state.read() == ThreadState::Parked {
        meta.unpark();
        meta.os_thread.unpark();
    } else {
        let mut unpark_count_lock = meta.unsafe_unpark_count.lock();
        *unpark_count_lock += 1;
    };
    non_failing_none()
});