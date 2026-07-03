use crate::access_flags::{FieldFlag, MethodFlag};
use crate::class_file::fields::{get_class_descriptor, FieldInfo};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::class_file::methods::{descriptor, INVALID_VTABLE_INDEX, NONVIRTUAL_VTABLE_INDEX};
use crate::vm::call_info::{resolve_virtual_call, CallInfo, CallInfoKind};
use crate::vm::class::ClassAndMethod;
use crate::vm::constants::classes::{JAVA_LANG_CLASS, JAVA_LANG_INVOKE_METHOD_HANDLE, JAVA_LANG_INVOKE_METHOD_TYPE, JAVA_LANG_INVOKE_MHN, JAVA_LANG_REFLECT_CONSTRUCTOR, JAVA_LANG_REFLECT_FIELD, JAVA_LANG_REFLECT_METHOD, JAVA_LANG_STRING};
use crate::vm::constants::{LAMBDAFORM_vmentry_INDEX, MEMBERNAME_clazz_INDEX, MEMBERNAME_flags_INDEX, MEMBERNAME_name_INDEX, MEMBERNAME_type_INDEX, METHODHANDLE_form_INDEX, METHODTYPE_ptypes_INDEX, METHODTYPE_rtype_INDEX, METHOD_clazz_INDEX, METHOD_slot_INDEX};
use crate::vm::java_error::JavaError;
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::result::{VMPartialResult, VMResult, VMResultType};
use crate::vm::value::{Reference, ReferenceType, Value};
use crate::vm::{Context, VmError, VM};
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_LANG_INVOKE_METHOD_HANDLE, "invoke", "([Ljava/lang/Object;)Ljava/lang/Object;", delegate_invoke);
    registry.register(JAVA_LANG_INVOKE_METHOD_HANDLE, "invokeBasic", "([Ljava/lang/Object;)Ljava/lang/Object;", delegate_invoke);
    registry.register(JAVA_LANG_INVOKE_METHOD_HANDLE, "invokeExact", "([Ljava/lang/Object;)Ljava/lang/Object;", delegate_invoke);
    registry.register(JAVA_LANG_INVOKE_METHOD_HANDLE, "linkToStatic", "([Ljava/lang/Object;)Ljava/lang/Object;", delegate_link_to_static);
    registry.register(JAVA_LANG_INVOKE_MHN, "getConstant", "(I)I", delegate_get_constant);
    registry.register(JAVA_LANG_INVOKE_MHN, "getNamedCon", "(I[Ljava/lang/Object;)I", delegate_get_named_con);
    registry.register(JAVA_LANG_INVOKE_MHN, "objectFieldOffset", "(Ljava/lang/invoke/MemberName;)J", delegate_object_field_offset);
    registry.register(JAVA_LANG_INVOKE_MHN, "getMembers", "(Ljava/lang/Class;Ljava/lang/String;Ljava/lang/String;ILjava/lang/Class;I[Ljava/lang/invoke/MemberName;)I", delegate_get_members);
    registry.register(JAVA_LANG_INVOKE_MHN, "getMemberVMInfo", "(Ljava/lang/invoke/MemberName;)Ljava/lang/Object;", delegate_get_member_vminfo);
    registry.register(JAVA_LANG_INVOKE_MHN, "init", "(Ljava/lang/invoke/MemberName;Ljava/lang/Object;)V", delegate_init);
    registry.register(JAVA_LANG_INVOKE_MHN, "resolve", "(Ljava/lang/invoke/MemberName;Ljava/lang/Class;)Ljava/lang/invoke/MemberName;", delegate_resolve);
}

gen_delegate!(delegate_invoke, |ctx, obj_ref, args| {
    if let Some(mh_ref) = obj_ref {
        // form
        let lambda_form_ref = ctx.vm.resolve_object_by_id(mh_ref.get_ref_field(METHODHANDLE_form_INDEX)?)?;
        // vmentry
        let vmentry_ref = ctx.vm.resolve_object_by_id(lambda_form_ref.get_ref_field(LAMBDAFORM_vmentry_INDEX)?)?;
        let blub = ctx.vm.object_payloads.read()?.get(&vmentry_ref.id).unwrap().clone();
        if let (Some(Value::Reference(vmtarget_ref_id)), Some(Value::Reference(mname_ref_id))) = (blub.get(0), blub.get(1)) {
            let vmtarget = ctx.vm.extract_long(Value::Reference(*vmtarget_ref_id))?.expect_long()?;
            let mname_ref = ctx.vm.resolve_object_by_id(*mname_ref_id)?;
            let clazz = ctx.vm.resolve_clazz_by_class_ref_id(mname_ref.get_ref_field(MEMBERNAME_clazz_INDEX)?)?;
            let name = ctx.vm.extract_string_from_value(mname_ref.get_field(MEMBERNAME_name_INDEX))?;

            if vmtarget as isize == NONVIRTUAL_VTABLE_INDEX {
                let typ_ref = ctx.vm.resolve_object_by_id(mname_ref.get_ref_field(MEMBERNAME_type_INDEX)?)?;

                let desc = ctx.vm.extract_descriptor_from_method_type(typ_ref)?;

                let method = clazz.find_method(name.as_str(), desc.as_str()).unwrap();
                let cam = ClassAndMethod { class: clazz, method };
                assert!(cam.method.flags & MethodFlag::Static as u16 > 0);
                let mut delegate_args = vec![Value::Reference(mh_ref.id)];
                delegate_args.extend(args);
                let result = JavaThread::invoke_subroutine(ctx, cam, None, delegate_args);
                return match result{
                    Ok(any) => Ok(any),
                    Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
                    Err(e) => Err(e),
                };
            } else {
                todo!()
            }
        }
        unimplemented!()
    } else {
        invalidation!("Expected a MethodHandle object reference")
    }
});

gen_delegate!(delegate_link_to_static, |ctx, _obj_ref, args| {
    // this apparently just casts the simplified arguments back up to their target types
    // see: https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/opto/callGenerator.cpp#L807
    // FIXME check if the membername should stay in last or should be removed
    if let Some(Value::Reference(mname_ref_id)) = args.get(args.len() - 1) {
        let mname_ref = ctx.vm.resolve_object_by_id(*mname_ref_id)?;
        let typ_ref = ctx.vm.resolve_object_by_id(mname_ref.get_ref_field(MEMBERNAME_type_INDEX)?)?;
        let clazz = ctx.vm.resolve_clazz_by_class_ref_id(mname_ref.get_ref_field(MEMBERNAME_clazz_INDEX)?)? ;
        let name = ctx.vm.extract_string_from_value(mname_ref.get_field(MEMBERNAME_name_INDEX))?;
        println!("LTS: {}", name);

        let desc = ctx.vm.extract_descriptor_from_method_type(typ_ref)?;
        let desc = MethodDescriptor::new(desc);
        if desc.args.len() != args.len() - 1 {
            unreachable!("Args count does not match: expected: {}, got: {:?}", desc.as_str(), args)
        }

        let method = clazz.find_method(name.as_str(), desc.as_str()).unwrap();
        let cam = ClassAndMethod { class: clazz, method: method.clone() };

        let args_only = args[..args.len() - 1].iter().cloned().collect::<Vec<_>>();
        let result = JavaThread::invoke_subroutine(ctx, cam, None, args_only);
        return match result {
            Ok(any) => Ok(any),
            Err(VmError::JavaException(JavaError::JavaExceptionThrown(..))) => Ok(VMResultType::ExceptionThrown),
            Err(e) => Err(e),
        }
    }
    unimplemented!()
});

const GC_COUNT_GWT: i32 = 4;
const GC_LAMBDA_SUPPORT: i32 = 5;

gen_delegate!(delegate_get_constant, |_ctx, _obj_ref, args| {
    if let Some(Value::Integer(val)) = args.get(0){
        match *val{
            GC_COUNT_GWT => non_failing_some(Value::Integer(0)),
            _ => non_failing_some(Value::Integer(0))
        }
    } else {
        invalidation!("Expected integer argument")
    }
});

gen_delegate!(delegate_get_named_con, |_ctx, _obj_ref, args| {
    if let (Some(Value::Integer(_which)), Some(Value::Reference(_object_arr_ref))) = (args.get(0), args.get(1)){
        //TODO see https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/prims/methodHandles.cpp#L1115
        non_failing_some(Value::Integer(0))
    } else {
        invalidation!("Expected integer and object arr argument")
    }
});


gen_delegate!(delegate_object_field_offset, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(mname_id)) = args.get(0) {
        if !mname_id.is_null() && let Some(payload) = ctx.vm.object_payloads.read()?.get(&mname_id) {
            let mname_ref = ctx.vm.resolve_object_by_id(*mname_id)?;
            // flags
            let flags = mname_ref.get_int_field(MEMBERNAME_flags_INDEX)?;
            if flags & IS_FIELD != 0 && flags & FieldFlag::Static as i32 == 0 {
                non_failing_some(ctx.vm.extract_long(payload[0].clone())?)
            } else {
                invalidation!("member name does not represent a non-static field")
            }
        } else {
            invalidation!("member name is null or has no payload")
        }
    } else {
        invalidation!("expected member name reference but got: {:?}", args)
    }
});

gen_delegate!(delegate_get_members, |ctx, _obj_ref, args| {
    if let (
        Some(Value::Reference(class_ref_id)),
        Some(Value::Reference(name_ref_id)),
        Some(Value::Reference(sig_ref_id)),
        Some(Value::Integer(m_flags)),
        Some(Value::Reference(caller_ref_id)),
        Some(Value::Integer(skip)),
        Some(Value::Reference(results_ref_id)),
    ) = (args.get(0), args.get(1), args.get(2), args.get(3), args.get(4), args.get(5), args.get(6)) {
        if class_ref_id.is_null() || results_ref_id.is_null() {
            return non_failing_some(Value::Integer(-1))
        }
        let clazz = ctx.vm.resolve_clazz_by_class_ref_id(*class_ref_id)?;
        if name_ref_id.is_null() || sig_ref_id.is_null() {
            return non_failing_some(Value::Integer(0));
        }
        let name = ctx.vm.extract_string_from_value(Value::Reference(*name_ref_id))?;
        let sig = ctx.vm.extract_string_from_value(Value::Reference(*sig_ref_id))?;
        let caller = ctx.vm.resolve_clazz_by_class_ref_id(*caller_ref_id)?;
        todo!()
    }

    todo!()
});

gen_delegate!(delegate_get_member_vminfo, |ctx, _obj_ref, args| {
    if let Some(Value::Reference(self_id)) = args.get(0){
        if let Some(vals) = ctx.vm.object_payloads.read()?.get(&self_id){
            let array = wrap_init!(ctx, ctx.vm.new_object_array_1(vals.clone())?);
            non_failing_some(Value::Reference(array.id))
        } else {
            invalidation!("No vminfo payload found")
        }
    } else {
        invalidation!("Expected MemberName reference")
    }
});

const IS_METHOD: i32      = 0x00010000;
const IS_CONSTRUCTOR: i32 = 0x00020000;
const IS_FIELD: i32       = 0x00040000;
const IS_TYPE: i32        = 0x00080000;
const ALL_KINDS: i32 = IS_METHOD | IS_CONSTRUCTOR | IS_FIELD | IS_TYPE;
const REFERENCE_KIND_SHIFT: i32 = 24;

const REF_None: i32             = 0;
const REF_getField: i32         = 1;
const REF_getStatic: i32        = 2;
const putField: i32             = 3;
const putStatic: i32            = 4;
const REF_invokeVirtual: i32    = 5;
const REF_invokeStatic: i32     = 6;
const REF_invokeSpecial: i32    = 7;
const REF_newInvokeSpecial: i32 = 8;
const REF_invokeInterface: i32  = 9;

gen_delegate!(delegate_init, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(mname_id)), Some(Value::Reference(target_id))) = (args.get(0), args.get(1)){
        if !mname_id.is_null() && !target_id.is_null(){
            // see https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/prims/methodHandles.cpp#L129
            let target_ref = ctx.vm.resolve_object_by_id(*target_id)?;
            let mname_ref = ctx.vm.resolve_object_by_id(*mname_id)?;
            if target_ref.class_name == JAVA_LANG_REFLECT_METHOD {
                // clazz
                let class_ref_id = target_ref.get_ref_field(METHOD_clazz_INDEX)?;
                // slot
                let slot = target_ref.get_int_field(METHOD_slot_INDEX)?;

                let class_ref = ctx.vm.resolve_clazz_by_class_ref_id(class_ref_id)?;
                let method_info = class_ref.get_method_in_slot(slot as usize).unwrap();
                let info = CallInfo::new(&method_info, &class_ref);

                // clazz
                mname_ref.set_field(MEMBERNAME_clazz_INDEX, Value::Reference(class_ref_id));
                member_name_init_method(ctx, mname_ref, &info)?;

                non_failing_none()
            } else if target_ref.class_name == JAVA_LANG_REFLECT_FIELD {
                todo!()
            } else if target_ref.class_name == JAVA_LANG_REFLECT_CONSTRUCTOR {
                todo!()
            } else {
                invalidation!("target is not a valid type (expected Method, Field, Constructor, but got: {})", target_ref.class_name)
            }
        } else {
            invalidation!("MemberName or target reference are null")
        }
    } else {
        invalidation!("Expected MemberName and target reference")
    }
});

fn member_name_init_method<'a>(ctx: Context<'a, '_>, mname: Reference<'a>, info: &CallInfo<'a>) -> VMResult<()> {
    let m = info.resolved_method;
    let method_flags = m.flags as i32;
    let mut flags = method_flags;
    let mut vmindex = INVALID_VTABLE_INDEX;
    match info.kind {
        CallInfoKind::Itable => {
            vmindex = info.index;
            flags |= IS_METHOD | (REF_invokeInterface << REFERENCE_KIND_SHIFT);
        }
        CallInfoKind::Vtable => {
            vmindex = info.index;
            assert!(vmindex >= 0, "Invalid vtable index: {} in {:?}", vmindex, m);
            flags |= IS_METHOD | (REF_invokeVirtual << REFERENCE_KIND_SHIFT);
        }
        CallInfoKind::Direct => {
            vmindex = NONVIRTUAL_VTABLE_INDEX;
            if m.is_static() {
                flags |= IS_METHOD | (REF_invokeStatic << REFERENCE_KIND_SHIFT);
            } else if m.is_initializer() {
                flags |= IS_CONSTRUCTOR | (REF_invokeSpecial << REFERENCE_KIND_SHIFT);
            } else {
                flags |= IS_METHOD | (REF_invokeSpecial << REFERENCE_KIND_SHIFT);
            }
        }
        CallInfoKind::Unknown => return invalidation!("Unknown CallInfo kind")
    }

    // flags
    mname.set_field(MEMBERNAME_flags_INDEX, Value::Integer(flags));
    // vmindex
    // vmtarget
    let vmindex = ctx.vm.new_java_lang_long(Value::Long(vmindex as i64))?;
    let old = ctx.vm.object_payloads.write()?.insert(mname.id, vec![vmindex, Value::Reference(mname.id)]);
    Ok(())
}

fn member_name_init_field<'a>(ctx: Context<'a, '_>, mname: Reference<'a>, field_info: &FieldInfo) -> VMResult<()> {
    let mut flags = field_info.flags as i32;
    flags |= IS_FIELD | ((if field_info.is_static() { REF_getStatic } else { REF_getField } ) << REFERENCE_KIND_SHIFT);
    //TODO add support for setters

    let clazz = ctx.vm.find_class_by_id(field_info.holder_id).unwrap();
    let vmtarget = wrap_init!(ctx, ctx.vm.new_class_object_by_class(clazz)?);
    let slot = field_info.slot as i32;
    let vmindex = ctx.vm.new_java_lang_long(Value::Long(slot as i64))?;
    let old = ctx.vm.object_payloads.write()?.insert(mname.id, vec![vmindex, Value::Reference(vmtarget.id)]);

    // flags
    mname.set_field(MEMBERNAME_flags_INDEX, Value::Integer(flags));
    Ok(())
}


fn resolve_signature<'a>(ctx: Context<'a, '_>, sig: Reference<'a>) -> VMResult<String> {
    if sig.class_name == JAVA_LANG_INVOKE_METHOD_TYPE {
        let args_ref = ctx.vm.resolve_object_by_id(sig.get_ref_field(METHODTYPE_ptypes_INDEX)?)?;
        let rtype_ref_id = sig.get_ref_field(METHODTYPE_rtype_INDEX)?;
        if let ReferenceType::Array(_, _, content) = &args_ref.reference_type{
            let mut signature = String::from("(");
            for class_obj in content.read()?.iter(){
                let Value::Reference(class_ref_id) = *class_obj else { return invalidation!("Expected Reference") };
                let class_ref = ctx.vm.resolve_object_by_id(class_ref_id)?;
                let class_name = ctx.vm.extract_class_name_from_class_ref(&class_ref)?;
                signature.push_str(get_class_descriptor(class_name.as_str()).as_str());
            }
            signature.push_str(")");
            if rtype_ref_id.is_null(){
                signature.push_str("V");
            } else {
                let class_ref = ctx.vm.resolve_object_by_id(rtype_ref_id)?;
                let class_name = ctx.vm.extract_class_name_from_class_ref(&class_ref)?;
                signature.push_str(get_class_descriptor(class_name.as_str()).as_str())
            }
            Ok(signature)
        } else {
            invalidation!("Invalid signature (args is not an array)")
        }
    } else if sig.class_name == JAVA_LANG_CLASS {
        let class_name = ctx.vm.extract_class_name_from_class_ref(sig)?;
        let signature = get_class_descriptor(class_name.as_str());
        Ok(signature)
    } else if sig.class_name == JAVA_LANG_STRING {
        todo!()
    } else {
        invalidation!("Invalid signature type")
    }
}

gen_delegate!(delegate_resolve, |ctx, _obj_ref, args| {
    if let (Some(Value::Reference(self_ref_id)), Some(Value::Reference(caller_id))) = (args.get(0), args.get(1)){
        let self_ref = ctx.vm.resolve_object_by_id(*self_ref_id)?;
        let clazz = ctx.vm.resolve_clazz_by_class_ref_id(self_ref.get_ref_field(MEMBERNAME_clazz_INDEX)?)?;
        //let caller_clazz = vm.extract_class_from_class_object(caller)?; can be null apparently
        let name = ctx.vm.extract_string_from_value(self_ref.get_field(MEMBERNAME_name_INDEX))?;
        let typ = ctx.vm.resolve_object_by_id(self_ref.get_ref_field(MEMBERNAME_type_INDEX)?)?;
        let sig = resolve_signature(ctx, typ)?;
        let flags = &self_ref.get_field(MEMBERNAME_flags_INDEX).expect_int()?;
        let ref_kind = flags >> REFERENCE_KIND_SHIFT;
        match flags & ALL_KINDS {
            IS_METHOD => {
                let call_info = match ref_kind {
                    REF_invokeStatic => {
                        if let Some(method_info) = clazz.find_method(name.as_str(), sig.as_str()) {
                            CallInfo::new_static(clazz, method_info)
                        } else {
                            let exception_class = wrap_init!(ctx, ctx.get_or_initialize_class("java/lang/NoSuchMethodException")?);
                            let exception_message = format!("Method {}.{}{} not found", clazz.name, name, sig);
                            let origin = "java/lang/invoke/MethodHandleNatives.resolve(Ljava/lang/invoke/MemberName;Ljava/lang/Class;)Ljava/lang/invoke/MemberName;".to_string();
                            return JavaThread::throw(ctx, exception_class, exception_message, origin);
                        }
                    }
                    REF_invokeInterface => {
                        let cam = clazz.resolve_interface_method_virtual(name.as_str(), sig.as_str()).unwrap();
                        if !cam.method.has_itable_index(){
                            CallInfo::new_virtual(clazz, cam.class, cam.method, cam.method, cam.method.vtable_index())
                        } else {
                            CallInfo::new_interface(clazz, cam.class, cam.method, cam.method, cam.method.itable_index())
                        }
                    }
                    REF_invokeVirtual => {
                        resolve_virtual_call(clazz, clazz, name.as_str(), sig.as_str())
                    }
                    _ => unreachable!("Invalid ref_kind: {}", ref_kind)
                };
                member_name_init_method(ctx, self_ref, &call_info)?;
                //selff.set_field(3, Value::Integer(*flags | call_info.selected_method.flags as i32));
            }
            IS_FIELD => {
                let (vmindex, field_info, holder_id) = clazz.find_field_static(name.as_str()).unwrap();
                self_ref.set_field(MEMBERNAME_flags_INDEX, Value::Integer(*flags | field_info.flags as i32));
                member_name_init_field(ctx, self_ref, field_info)?;
            }
            other => todo!("Unsupported mh type: {:b}", other)
        };

        //TODO see: https://github.com/openjdk/jdk8u/blob/master/hotspot/src/share/vm/prims/methodHandles.cpp#L609

        //let prev = vm.object_payloads.borrow_mut().insert(selff.id, vec![vmindex, vmtarget]);
        //assert!(prev.is_none());
        non_failing_some(Value::Reference(self_ref.id))
    } else {
        invalidation!("Expected two references")
    }
});