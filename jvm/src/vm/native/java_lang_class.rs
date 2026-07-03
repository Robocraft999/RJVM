use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::class_file::fields::field_type::FieldType;
use crate::error::ClassParseError;
use crate::vm::constants::classes::{JAVA_LANG_CLASS, JAVA_LANG_REFLECT_CONSTRUCTOR, JAVA_LANG_REFLECT_FIELD, JAVA_LANG_REFLECT_METHOD};
use crate::vm::constants::{CONSTRUCTOR_clazz_INDEX, CONSTRUCTOR_exceptionTypes_INDEX, CONSTRUCTOR_modifiers_INDEX, CONSTRUCTOR_parameterTypes_INDEX, CONSTRUCTOR_slot_INDEX, FIELD_clazz_INDEX, FIELD_modifiers_INDEX, FIELD_name_INDEX, FIELD_type_INDEX, METHOD_clazz_INDEX, METHOD_exceptionTypes_INDEX, METHOD_modifiers_INDEX, METHOD_name_INDEX, METHOD_parameterTypes_INDEX, METHOD_returnType_INDEX, METHOD_slot_INDEX};
use crate::vm::native::{gen_delegate, invalidation, non_failing_none, non_failing_some, wrap_init, NativeMethodRegistry};
use crate::vm::value::ReferenceType;
use crate::vm::JavaVM;
use crate::vm::Reference;
use crate::vm::{VMPartialResult, VmError};
use crate::Value;
use crate::VM;
use log::{debug, info};
use std::cell::RefCell;
use std::sync::RwLock;
use crate::vm::java_thread::JavaThread;

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    let mut register = |method_name, sig, delegate|registry.register(JAVA_LANG_CLASS, method_name, sig, delegate);
    register("getPrimitiveClass", "(Ljava/lang/String;)Ljava/lang/Class;", delegate_get_primitive_class);
    register("getComponentType", "()Ljava/lang/Class;", delegate_get_component_type);
    register("getClassLoader0", "()Ljava/lang/ClassLoader;", delegate_get_classloader0);
    register("getProtectionDomain0", "()Ljava/security/ProtectionDomain;", delegate_get_protection_domain0);
    register("desiredAssertionStatus0", "(Ljava/lang/Class;)Z", delegate_desired_assertion_status0);
    register("getDeclaredFields0", "(Z)[Ljava/lang/reflect/Field;", delegate_get_declared_fields0);
    register("getDeclaredConstructors0", "(Z)[Ljava/lang/reflect/Constructor;", delegate_get_declared_constructors0);
    register("getDeclaredMethods0", "(Z)[Ljava/lang/reflect/Method;", delegate_get_declared_methods0);
    register("getModifiers", "()I", delegate_get_modifiers);
    register("getSuperclass", "()Ljava/lang/Class;", delegate_get_superclass);
    register("getEnclosingMethod0", "()[Ljava/lang/Object;", delegate_get_enclosing_method0);
    register("getDeclaringClass0", "()Ljava/lang/Class;", delegate_get_declaring_class0);
    register("getDeclaredClasses0", "()[Ljava/lang/Class;", delegate_get_declared_classes0);
    register("forName0", "(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;", delegate_for_name0);
    register("isInterface", "()Z", delegate_is_interface);
    register("isArray", "()Z", delegate_is_array);
    register("isPrimitive", "()Z", delegate_is_primitive);
    register("isAssignableFrom", "(Ljava/lang/Class;)Z", delegate_is_assignable_from);
}

gen_delegate!(delegate_get_primitive_class, |ctx, _obj, args| {
    let string = ctx.vm.extract_string_from_value(*args.get(0).unwrap())?;
    let class_id = ctx.vm.class_manager.get_primitive_class(&ctx.vm, string.as_str());
    match string.as_str() {
        "int"     |
        "long"    |
        "short"   |
        "char"    |
        "byte"    |
        "float"   |
        "double"  |
        "boolean" |
        "void"    => non_failing_some(Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object(string.as_str(), class_id)?).id)),
        _ => invalidation!("Expected extractable string")
    }
});

gen_delegate!(delegate_get_component_type, |ctx, class_ref, args| {
    debug!("getComponentType \n'{:?}'\n'{:?}'", class_ref, args);
    let class_name = ctx.vm.extract_class_name_from_class_ref(class_ref.unwrap())?;
    //let field_type = field_type_from_str(class_name.as_str());
    debug!("getComponentType '{:?}'", class_name);

    let array_class = ctx.vm.get_or_resolve_class(class_name.as_str())?;
    if let Some(array_info) = &array_class.array_info{
        let component_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_from_field_type(&array_info.component_type)?);
        non_failing_some(Value::Reference(component_class_ref.id))
    } else {
        invalidation!("Expected Array object but found '{:?}'", class_ref)
    }
});

gen_delegate!(delegate_get_classloader0, |ctx, _class_object, _args| {
    //TODO check
    debug!("getClassLoader0");
    non_failing_some(ctx.vm.null())
});

gen_delegate!(delegate_get_protection_domain0, |ctx, _class_object, _args| {
    debug!("getProtectionDomain0");
    non_failing_some(ctx.vm.null())
});

gen_delegate!(delegate_desired_assertion_status0, |_ctx, _class_object, _args| {
    //TODO check
    debug!("desiredAssertionStatus0");
    non_failing_some(Value::Integer(1))
});

gen_delegate!(delegate_get_declared_fields0, |ctx, class_ref, _args| {
    debug!("getDeclaredFields");
    if let Some(class_ref) = class_ref {
        let class_name = ctx.vm.extract_class_name_from_class_ref(class_ref)?;
        debug!("class name: {}", class_name);
        let clazz = ctx.vm.get_or_resolve_class(class_name.as_str())?;
        let mut content = Vec::new();
        for field in clazz.fields.iter(){
            let java_field = wrap_init!(ctx, ctx.new_object(JAVA_LANG_REFLECT_FIELD)?);
            //name
            java_field.set_field(FIELD_name_INDEX, Value::Reference(wrap_init!(ctx, ctx.vm.new_string_object(field.name.as_str())?).id));
            //clazz
            java_field.set_field(FIELD_clazz_INDEX, Value::Reference(class_ref.id));
            //modifiers
            java_field.set_field(FIELD_modifiers_INDEX, Value::Integer(field.flags as i32));
            //type
            let type_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_from_field_type(&field.field_type)?);
            java_field.set_field(FIELD_type_INDEX, Value::Reference(type_class_ref.id));
            info!("field name: {}", field.name);
            content.push(Value::Reference(java_field.id));
        }
        let fields_arr_ref = wrap_init!(ctx, ctx.vm.new_array(1, FieldType::Object(JAVA_LANG_REFLECT_FIELD.to_string()).to_array_field_type(1), RwLock::new(content.clone()))?);
        non_failing_some(Value::Reference(fields_arr_ref.id))
    } else {
        //FIXME i dont know if this should be none
        non_failing_none()
    }
});

gen_delegate!(delegate_get_declared_constructors0, |ctx, class_ref, args| {
    debug!("getDeclaredConstructors0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_ref, args.get(0)){
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        let java_constructor_class = wrap_init!(ctx, ctx.get_or_initialize_class(JAVA_LANG_REFLECT_CONSTRUCTOR)?);
        let mut content = Vec::new();
        for constructor in clazz.get_constructors(*public_only == 1).iter(){
            let java_constructor_ref = ctx.vm.new_object_from_class(java_constructor_class);

            // clazz
            java_constructor_ref.set_field(CONSTRUCTOR_clazz_INDEX, Value::Reference(class_ref.id));

            // slot
            java_constructor_ref.set_field(CONSTRUCTOR_slot_INDEX, Value::Integer(constructor.slot as i32));

            let mut parameters = Vec::new();
            for field_type in constructor.descriptor.args.iter(){
                let parameter_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_from_field_type(field_type)?);
                parameters.push(Value::Reference(parameter_class_ref.id));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = &constructor.attributes.exceptions {
                for exception_index in &exception_vec.exception_index_table {
                    let exception_clazz = if let Some(ConstantPoolEntry::Class(clazz)) = clazz.get_or_resolve_constant(ctx.vm, *exception_index) {
                        Ok(clazz)
                    } else {
                        invalidation!("Exception class could not be resolved in class: {}", clazz.name)
                    }?;
                    let parameter_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_by_class(exception_clazz)?);
                    exceptions.push(Value::Reference(parameter_class_ref.id));
                }
            }
            // parameterTypes
            java_constructor_ref.set_field(CONSTRUCTOR_parameterTypes_INDEX, Value::Reference(wrap_init!(ctx, ctx.vm.new_class_array_1(parameters.clone())?).id));
            // exceptionTypes
            java_constructor_ref.set_field(CONSTRUCTOR_exceptionTypes_INDEX, Value::Reference(wrap_init!(ctx, ctx.vm.new_class_array_1(exceptions.clone())?).id));

            // modifiers
            java_constructor_ref.set_field(CONSTRUCTOR_modifiers_INDEX, Value::Integer(constructor.flags as i32));

            content.push(Value::Reference(java_constructor_ref.id));
        }
        let contructor_arr_ref = wrap_init!(ctx, ctx.vm.new_array(1, FieldType::Object(JAVA_LANG_REFLECT_CONSTRUCTOR.to_string()).to_array_field_type(1), RwLock::new(content.clone()))?);
        non_failing_some(Value::Reference(contructor_arr_ref.id))
    } else {
        invalidation!("Expected Class object and boolean")
    }
});

gen_delegate!(delegate_get_declared_methods0, |ctx, class_ref, args| {
    debug!("getDeclaredMethods0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_ref, args.get(0)){
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        let mut content = Vec::new();
        for method in clazz.get_methods(*public_only == 1).iter(){
            let java_method_ref = wrap_init!(ctx, ctx.new_object(JAVA_LANG_REFLECT_METHOD)?);

            // clazz
            java_method_ref.set_field(METHOD_clazz_INDEX, Value::Reference(class_ref.id));

            // slot
            java_method_ref.set_field(METHOD_slot_INDEX, Value::Integer(method.slot as i32));

            let name = wrap_init!(ctx, ctx.vm.new_string_object(&method.name.as_str())?);
            // name
            java_method_ref.set_field(METHOD_name_INDEX, Value::Reference(name.id));

            let return_type = if let Some(f) = &method.descriptor.return_type{
                Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object_from_field_type(f)?).id)
            } else {
                Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object("void", ctx.vm.class_manager.get_primitive_class(ctx.vm, "void"))?).id)
            };
            let mut parameters = Vec::new();
            for field_type in method.descriptor.args.iter(){
                let parameter_class = wrap_init!(ctx, ctx.vm.new_class_object_from_field_type(field_type)?);
                parameters.push(Value::Reference(parameter_class.id));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = &method.attributes.exceptions {
                for exception_index in &exception_vec.exception_index_table {
                    let exception_class = if let Some(ConstantPoolEntry::Class(clazz)) = clazz.get_or_resolve_constant(ctx.vm, *exception_index) {
                        Ok(clazz)
                    } else {
                        invalidation!("Exception class could not be resolved in class: {}", clazz.name)
                    }?;
                    let parameter_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_by_class(exception_class)?);
                    exceptions.push(Value::Reference(parameter_class_ref.id));
                }
            }

            // returnType
            java_method_ref.set_field(METHOD_returnType_INDEX, return_type);

            // parameterTypes
            java_method_ref.set_field(METHOD_parameterTypes_INDEX, Value::Reference(wrap_init!(ctx, ctx.vm.new_class_array_1(parameters.clone())?).id));

            // exceptionTypes
            java_method_ref.set_field(METHOD_exceptionTypes_INDEX, Value::Reference(wrap_init!(ctx, ctx.vm.new_class_array_1(exceptions.clone())?).id));

            // modifiers
            java_method_ref.set_field(METHOD_modifiers_INDEX, Value::Integer(method.flags as i32));

            content.push(Value::Reference(java_method_ref.id));
        }
        let method_arr_ref = wrap_init!(ctx, ctx.vm.new_array(1, FieldType::Object(JAVA_LANG_REFLECT_METHOD.to_string()).to_array_field_type(1), RwLock::new(content.clone()))?);
        non_failing_some(Value::Reference(method_arr_ref.id))
    } else {
        invalidation!("Expected Class object and boolean")
    }
});

gen_delegate!(delegate_get_modifiers, |ctx, class_ref, _args| {
    if let Some(class_ref) = class_ref{
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        let flags = clazz.flags as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_superclass, |ctx, class_ref, _args| {
    if let Some(class_ref) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        match clazz.superclass {
            Some(super_class) => {
                let super_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_by_name(super_class.name.as_str())?);
                non_failing_some(Value::Reference(super_class_ref.id))
            }
            None => non_failing_some(ctx.vm.null())
        }

    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_enclosing_method0, |ctx, class_ref, _args| {
    if let Some(class_ref) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        if let Some(enclosing) = &clazz.attributes.enclosing_method{
            let class_val = if let Some(ConstantPoolEntry::Class(enclosing_clazz)) = clazz.get_or_resolve_constant(ctx.vm, enclosing.class_index){
                Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object_by_class(enclosing_clazz)?).id)
            } else {
                return invalidation!("expected a class constant");
            };
            let (method_name, method_type) = if let Some(ConstantPoolEntry::NameAndType(name, typ)) = clazz.get_or_resolve_constant(ctx.vm, enclosing.class_index){
                (
                    Value::Reference(wrap_init!(ctx, ctx.vm.new_string_object(name.as_str())?).id),
                    Value::Reference(wrap_init!(ctx, ctx.vm.new_string_object(typ.as_str())?).id)
                )
            } else {
                return invalidation!("Expected NameAndType for EnclosingClass")
            };
            let res = wrap_init!(ctx, ctx.vm.new_object_array_1(vec![class_val.clone(), method_name.clone(), method_type.clone()])?);
            non_failing_some(Value::Reference(res.id))
        } else {
            non_failing_some(ctx.vm.null())
        }
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_declaring_class0, |ctx, class_ref, _args| {
    if let Some(obj) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(obj)?;
        if let Some(inner_classes) = &clazz.attributes.inner_classes{
            for inner_class in &inner_classes.classes {
                if let Some(ConstantPoolEntry::Class(inner_clazz)) = clazz.get_or_resolve_constant(ctx.vm, inner_class.inner_class_info_index) && clazz.name == inner_clazz.name{
                    if let Some(ConstantPoolEntry::Class(outer_clazz)) = clazz.get_or_resolve_constant(ctx.vm, inner_class.outer_class_info_index){
                        let outer_class_obj = wrap_init!(ctx, ctx.vm.new_class_object_by_class(outer_clazz)?);
                        return non_failing_some(Value::Reference(outer_class_obj.id));
                    }
                }
            }
        }
        non_failing_some(ctx.vm.null())
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_declared_classes0, |ctx, class_ref, _args| {
    if let Some(obj) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(obj)?;
        let mut inner = Vec::new();
        if let Some(inner_classes) = &clazz.attributes.inner_classes {
            for inner_classes_entry in inner_classes.classes.iter() {
                if inner_classes_entry.outer_class_info_index == 0 || inner_classes_entry.inner_class_info_index == 0 {
                    continue;
                }
                if let Some(ConstantPoolEntry::Class(outer_clazz)) = clazz.get_or_resolve_constant(ctx.vm, inner_classes_entry.outer_class_info_index) && clazz.name == outer_clazz.name {
                    if let Some(ConstantPoolEntry::Class(inner_clazz)) = clazz.get_or_resolve_constant(ctx.vm, inner_classes_entry.inner_class_info_index) {
                        let inner_class_ref = wrap_init!(ctx, ctx.vm.new_class_object_by_class(inner_clazz)?);
                        inner.push(Value::Reference(inner_class_ref.id));
                    }
                }
            }
        }
        let array_ref = wrap_init!(ctx, ctx.vm.new_class_array_1(inner.clone())?);
        non_failing_some(Value::Reference(array_ref.id))
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_for_name0, |ctx, _class_object, args| {
    debug!("forName0");
    let exception = |name: &str| {
        let exception_message = format!("Class '{}' was not found", name);
        let exception_class = wrap_init!(ctx, ctx.get_or_initialize_class("java/lang/ClassNotFoundException")?);
        JavaThread::throw(
            ctx,
            exception_class,
            exception_message,
            String::from("java/lang/Class.forName0(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;")
        )
    };

    if let Some(name) = args.get(0) && !name.is_null(){
        let name = ctx.vm.extract_string_from_value(*name)?;
        let name = name.replace(".", "/");
        match ctx.vm.get_or_resolve_class(&name){
            Ok(..) => {
                non_failing_some(Value::Reference(wrap_init!(ctx, ctx.vm.new_class_object_by_name(&name)?).id))
            },
            Err(VmError::ParseError(ClassParseError::ResolveError(_))) => {
                exception(name.as_str())
            }
            Err(err) => Err(err)
        }
    } else {
        exception("")
    }
});

gen_delegate!(delegate_is_interface, |ctx, class_ref, _args| {
    debug!("isInterface {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        non_failing_some(Value::from(clazz.is_interface()))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_array, |ctx, class_ref, _args| {
    debug!("isArray {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let clazz = ctx.vm.extract_class_from_class_object(class_ref)?;
        non_failing_some(Value::from(clazz.is_array()))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_primitive, |ctx, class_ref, _args| {
    debug!("isPrimitive {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let name = ctx.vm.extract_class_name_from_class_ref(class_ref)?;
        non_failing_some(Value::Integer(match name.as_str() {
            "boolean" | "char" | "byte" | "short" | "int" | "long" | "float" | "double" | "void" => 1,
            _ => 0,
        }))
        //Ok(Some(Value::Integer(if PrimitiveType::from_str(name.as_str()).is_ok() { 1 } else { 0 })))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_assignable_from, |ctx, class_ref, args| {
    debug!("isAssignableFrom\nthis: {:?}\nfrom: {:?}", class_ref, args);
    if let (Some(class_ref), Some(Value::Reference(other_id))) = (class_ref, args.get(0)) {
        let this_class = ctx.vm.extract_class_from_class_object(class_ref)?;
        let from_class = ctx.vm.resolve_clazz_by_class_ref_id(*other_id)?;
        non_failing_some(Value::from(ctx.vm.unchecked_check_if_subclass_of(this_class.name.as_str(), from_class.name.as_str())?))
    } else {
        invalidation!("expected a class reference")
    }
});