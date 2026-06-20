use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::class_file::fields::field_type::FieldType;
use crate::error::ClassParseError;
use crate::vm::constants::classes::JAVA_LANG_CLASS;
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

gen_delegate!(delegate_get_primitive_class, |vm, java_vm, _obj, args| {
    let string = VM::extract_string_from_object(args.get(0).unwrap())?;
    let class_id = vm.class_manager.get_primitive_class(&vm, string.as_str());
    match string.as_str() {
        "int"     |
        "long"    |
        "short"   |
        "char"    |
        "byte"    |
        "float"   |
        "double"  |
        "boolean" |
        "void"    => non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object(string.as_str(), class_id)?))),
        _ => invalidation!("Expected extractable string")
    }
});

gen_delegate!(delegate_get_component_type, |vm, java_vm, class_ref, args| {
    debug!("getComponentType \n'{:?}'\n'{:?}'", class_ref, args);
    let class_name = VM::extract_class_name_from_class_object(class_ref.unwrap())?;
    //let field_type = field_type_from_str(class_name.as_str());
    debug!("getComponentType '{:?}'", class_name);

    let array_class = vm.get_or_resolve_class(class_name.as_str())?;
    if let Some(array_info) = &array_class.array_info{
        let component_class_ref = wrap_init!(vm, java_vm, vm.new_class_object_from_field_type(&array_info.component_type)?);
        non_failing_some(Value::Reference(component_class_ref))
    } else {
        invalidation!("Expected Array object but found '{:?}'", class_ref)
    }
});

gen_delegate!(delegate_get_classloader0, |vm, _java_vm, _class_object, _args| {
    //TODO check
    debug!("getClassLoader0");
    non_failing_some(vm.null())
});

gen_delegate!(delegate_get_protection_domain0, |vm, _java_vm, _class_object, _args| {
    debug!("getProtectionDomain0");
    non_failing_some(vm.null())
});

gen_delegate!(delegate_desired_assertion_status0, |_vm, _java_vm, _class_object, _args| {
    //TODO check
    debug!("desiredAssertionStatus0");
    non_failing_some(Value::Integer(1))
});

gen_delegate!(delegate_get_declared_fields0, |vm, java_vm, class_ref, _args| {
    debug!("getDeclaredFields");
    if let Some(class_ref) = class_ref {
        let class_name = VM::extract_class_name_from_class_object(class_ref)?;
        debug!("class name: {}", class_name);
        let clazz = vm.get_or_resolve_class(class_name.as_str())?;
        let mut content = Vec::new();
        for field in clazz.fields.iter(){
            let java_field = wrap_init!(vm, java_vm, vm.new_object("java/lang/reflect/Field")?);
            //name
            java_field.set_field(FIELD_name_INDEX, Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(field.name.as_str())?)));
            //clazz
            java_field.set_field(FIELD_clazz_INDEX, Value::Reference(class_ref));
            //modifiers
            java_field.set_field(FIELD_modifiers_INDEX, Value::Integer(field.flags as i32));
            //type
            let type_class_ref = wrap_init!(vm, java_vm, vm.new_class_object_from_field_type(&field.field_type)?);
            java_field.set_field(FIELD_type_INDEX, Value::Reference(type_class_ref));
            info!("field name: {}", field.name);
            content.push(Value::Reference(java_field));
        }
        for field in content.iter(){
            if let Value::Reference(java_field) = field {
                debug!("field : {:?}", java_field);
                if let ReferenceType::Object(fields) = &java_field.reference_type{
                    for field_field in fields.borrow().iter(){
                        debug!("field_: {:?}", field_field);
                    }
                }
            }
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Field".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        //FIXME i dont know if this should be none
        non_failing_none()
    }
});

gen_delegate!(delegate_get_declared_constructors0, |vm, java_vm, class_ref, args| {
    debug!("getDeclaredConstructors0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_ref, args.get(0)){
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        let java_constructor_class = wrap_init!(vm, java_vm, vm.get_or_initialize_class("java/lang/reflect/Constructor")?);
        let mut content = Vec::new();
        for constructor in clazz.get_constructors(*public_only == 1).iter(){
            let java_constructor = vm.new_object_from_class(java_constructor_class);

            // clazz
            java_constructor.set_field(CONSTRUCTOR_clazz_INDEX, Value::Reference(class_ref));

            // slot
            java_constructor.set_field(CONSTRUCTOR_slot_INDEX, Value::Integer(constructor.slot as i32));

            let mut parameters = Vec::new();
            for field_type in constructor.descriptor.args.iter(){
                let parameter_class_ref = wrap_init!(vm, java_vm, vm.new_class_object_from_field_type(field_type)?);
                parameters.push(Value::Reference(parameter_class_ref));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = &constructor.attributes.exceptions {
                for exception_index in &exception_vec.exception_index_table {
                    let exception_clazz = if let Some(ConstantPoolEntry::Class(clazz)) = clazz.get_or_resolve_constant(vm, *exception_index) {
                        Ok(clazz)
                    } else {
                        invalidation!("Exception class could not be resolved in class: {}", clazz.name)
                    }?;
                    let parameter_class_ref = wrap_init!(vm, java_vm, vm.new_class_object_by_class(exception_clazz)?);
                    exceptions.push(Value::Reference(parameter_class_ref));
                }
            }
            // parameterTypes
            java_constructor.set_field(CONSTRUCTOR_parameterTypes_INDEX, Value::Reference(wrap_init!(vm, java_vm, vm.new_class_array_1(parameters.clone())?)));
            // exceptionTypes
            java_constructor.set_field(CONSTRUCTOR_exceptionTypes_INDEX, Value::Reference(wrap_init!(vm, java_vm, vm.new_class_array_1(exceptions.clone())?)));

            // modifiers
            java_constructor.set_field(CONSTRUCTOR_modifiers_INDEX, Value::Integer(constructor.flags as i32));

            content.push(Value::Reference(java_constructor));
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Constructor".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        invalidation!("Expected Class object and boolean")
    }
});

gen_delegate!(delegate_get_declared_methods0, |vm, java_vm, class_ref, args| {
    debug!("getDeclaredMethods0");
    if let (Some(class_ref), Some(Value::Integer(public_only))) = (class_ref, args.get(0)){
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        let mut content = Vec::new();
        for method in clazz.get_methods(*public_only == 1).iter(){
            let java_method = wrap_init!(vm, java_vm, vm.new_object("java/lang/reflect/Method")?);

            // clazz
            java_method.set_field(METHOD_clazz_INDEX, Value::Reference(class_ref));

            // slot
            java_method.set_field(METHOD_slot_INDEX, Value::Integer(method.slot as i32));

            let name = wrap_init!(vm, java_vm, vm.new_string_object(&method.name.as_str())?);
            // name
            java_method.set_field(METHOD_name_INDEX, Value::Reference(name));

            let return_type = if let Some(f) = &method.descriptor.return_type{
                Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_from_field_type(f)?))
            } else {
                Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object("void", vm.class_manager.get_primitive_class(vm, "void"))?))
            };
            let mut parameters = Vec::new();
            for field_type in method.descriptor.args.iter(){
                let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_from_field_type(field_type)?);
                parameters.push(Value::Reference(parameter_class));
            }
            let mut exceptions = Vec::new();
            if let Some(exception_vec) = &method.attributes.exceptions {
                for exception_index in &exception_vec.exception_index_table {
                    let exception_class = if let Some(ConstantPoolEntry::Class(clazz)) = clazz.get_or_resolve_constant(vm, *exception_index) {
                        Ok(clazz)
                    } else {
                        invalidation!("Exception class could not be resolved in class: {}", clazz.name)
                    }?;
                    let parameter_class = wrap_init!(vm, java_vm, vm.new_class_object_by_class(exception_class)?);
                    exceptions.push(Value::Reference(parameter_class));
                }
            }

            // returnType
            java_method.set_field(METHOD_returnType_INDEX, return_type);

            // parameterTypes
            java_method.set_field(METHOD_parameterTypes_INDEX, Value::Reference(wrap_init!(vm, java_vm, vm.new_class_array_1(parameters.clone())?)));

            // exceptionTypes
            java_method.set_field(METHOD_exceptionTypes_INDEX, Value::Reference(wrap_init!(vm, java_vm, vm.new_class_array_1(exceptions.clone())?)));

            // modifiers
            java_method.set_field(METHOD_modifiers_INDEX, Value::Integer(method.flags as i32));

            content.push(Value::Reference(java_method));
        }
        non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_array(1, FieldType::Object("java/lang/reflect/Method".to_string()).to_array_field_type(1), RefCell::new(content.clone()))?)))
    } else {
        invalidation!("Expected Class object and boolean")
    }
});

gen_delegate!(delegate_get_modifiers, |vm, _java_vm, class_ref, _args| {
    if let Some(class_ref) = class_ref{
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        let flags = clazz.flags as i32;
        non_failing_some(Value::Integer(flags))
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_superclass, |vm, java_vm, class_ref, _args| {
    if let Some(class_ref) = class_ref {
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        match clazz.superclass {
            Some(super_class) => {
                let super_class_object = wrap_init!(vm, java_vm, vm.new_class_object_by_name(super_class.name.as_str())?);
                non_failing_some(Value::Reference(super_class_object))
            }
            None => non_failing_some(vm.null())
        }

    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_enclosing_method0, |vm, java_vm, class_ref, _args| {
    if let Some(class_ref) = class_ref {
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        if let Some(enclosing) = &clazz.attributes.enclosing_method{
            let class_val = if let Some(ConstantPoolEntry::Class(enclosing_clazz)) = clazz.get_or_resolve_constant(vm, enclosing.class_index){
                Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_class(enclosing_clazz)?))
            } else {
                return invalidation!("expected a class constant");
            };
            let (method_name, method_type) = if let Some(ConstantPoolEntry::NameAndType(name, typ)) = clazz.get_or_resolve_constant(vm, enclosing.class_index){
                (
                    Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(name.as_str())?)),
                    Value::Reference(wrap_init!(vm, java_vm, vm.new_string_object(typ.as_str())?))
                )
            } else {
                return invalidation!("Expected NameAndType for EnclosingClass")
            };
            let res = wrap_init!(vm, java_vm, vm.new_object_array_1(vec![class_val.clone(), method_name.clone(), method_type.clone()])?);
            non_failing_some(Value::Reference(res))
        } else {
            non_failing_some(vm.null())
        }
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_declaring_class0, |vm, java_vm, class_ref, _args| {
    if let Some(obj) = class_ref {
        let clazz = vm.extract_class_from_class_object(obj)?;
        if let Some(inner_classes) = &clazz.attributes.inner_classes{
            for inner_class in &inner_classes.classes {
                if let Some(ConstantPoolEntry::Class(inner_clazz)) = clazz.get_or_resolve_constant(vm, inner_class.inner_class_info_index) && clazz.name == inner_clazz.name{
                    if let Some(ConstantPoolEntry::Class(outer_clazz)) = clazz.get_or_resolve_constant(vm, inner_class.outer_class_info_index){
                        let outer_class_obj = wrap_init!(vm, java_vm, vm.new_class_object_by_class(outer_clazz)?);
                        return non_failing_some(Value::Reference(outer_class_obj));
                    }
                }
            }
        }
        non_failing_some(vm.null())
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_get_declared_classes0, |vm, java_vm, class_ref, _args| {
    if let Some(obj) = class_ref {
        let clazz = vm.extract_class_from_class_object(obj)?;
        let mut inner = Vec::new();
        if let Some(inner_classes) = &clazz.attributes.inner_classes {
            for inner_classes_entry in inner_classes.classes.iter() {
                if inner_classes_entry.outer_class_info_index == 0 || inner_classes_entry.inner_class_info_index == 0 {
                    continue;
                }
                if let Some(ConstantPoolEntry::Class(outer_clazz)) = clazz.get_or_resolve_constant(vm, inner_classes_entry.outer_class_info_index) && clazz.name == outer_clazz.name {
                    if let Some(ConstantPoolEntry::Class(inner_clazz)) = clazz.get_or_resolve_constant(vm, inner_classes_entry.inner_class_info_index) {
                        let inner_class_obj = wrap_init!(vm, java_vm, vm.new_class_object_by_class(inner_clazz)?);
                        inner.push(Value::Reference(inner_class_obj));
                    }
                }
            }
        }
        let array_ref = wrap_init!(vm, java_vm, vm.new_class_array_1(inner.clone())?);
        non_failing_some(Value::Reference(array_ref))
    } else {
        invalidation!("Expected Class object")
    }
});

gen_delegate!(delegate_for_name0, |vm, java_vm, _class_object, args| {
    debug!("forName0");
    let exception = |name: &str| {
        let exception_message = format!("Class '{}' was not found", name);
        let exception_class = wrap_init!(vm, java_vm, vm.get_or_initialize_class("java/lang/ClassNotFoundException")?);
        vm.throw(
            exception_class,
            exception_message,
            String::from("java/lang/Class.forName0(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)Ljava/lang/Class;")
        )
    };

    if let Some(name) = args.get(0) && !name.is_null(){
        let name = VM::extract_string_from_object(&name)?;
        let name = name.replace(".", "/");
        match vm.get_or_resolve_class(&name){
            Ok(..) => {
                non_failing_some(Value::Reference(wrap_init!(vm, java_vm, vm.new_class_object_by_name(&name)?)))
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

gen_delegate!(delegate_is_interface, |vm, _java_vm, class_ref, _args| {
    debug!("isInterface {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        non_failing_some(Value::from(clazz.is_interface()))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_array, |vm, _java_vm, class_ref, _args| {
    debug!("isArray {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let clazz = vm.extract_class_from_class_object(class_ref)?;
        non_failing_some(Value::from(clazz.is_array()))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_primitive, |_vm, _java_vm, class_ref, _args| {
    debug!("isPrimitive {:?}", class_ref);
    if let Some(class_ref) = class_ref {
        let name = VM::extract_class_name_from_class_object(class_ref)?;
        non_failing_some(Value::Integer(match name.as_str() {
            "boolean" | "char" | "byte" | "short" | "int" | "long" | "float" | "double" | "void" => 1,
            _ => 0,
        }))
        //Ok(Some(Value::Integer(if PrimitiveType::from_str(name.as_str()).is_ok() { 1 } else { 0 })))
    } else {
        invalidation!("this is Null")
    }
});

gen_delegate!(delegate_is_assignable_from, |vm, _java_vm, class_ref, args| {
    debug!("isAssignableFrom\nthis: {:?}\nfrom: {:?}", class_ref, args);
    if let (Some(class_ref), Some(Value::Reference(other_ref))) = (class_ref, args.get(0)) {
        let this_class = vm.extract_class_from_class_object(class_ref)?;
        let from_class = vm.extract_class_from_class_object(other_ref)?;
        non_failing_some(Value::from(vm.unchecked_check_if_subclass_of(this_class.name.as_str(), from_class.name.as_str())?))
    } else {
        invalidation!("expected a class reference")
    }
});