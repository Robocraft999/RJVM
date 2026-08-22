use log::error;
use crate::class_file::fields::field_type::FieldType;
use crate::vm::{Context, VmError};
use crate::vm::constants::classes::JAVA_LANG_OBJECT;
use crate::vm::result::VMResult;
use crate::vm::value::Value;

pub trait FieldTypeExt {
    fn validate(&self, value: Value, ctx: Context) -> VMResult<()>;
}

impl FieldTypeExt for FieldType {
    fn validate(&self, value: Value, ctx: Context) -> VMResult<()> {
        if !match self {
            FieldType::Object(class_name) => {
                if let Value::Reference(val_id) = value {
                    if val_id.is_null() {
                        true
                    } else {
                        let val_ref = ctx.vm.resolve_object_by_id(val_id)?;
                        let val_clazz = ctx.vm.find_class_by_id(val_ref.class_id);
                        let target_clazz = ctx.get_or_resolve_class(class_name);
                        if let ((Some(val_clazz)), Ok(target_clazz)) = (val_clazz, &target_clazz) {
                            if target_clazz.name == JAVA_LANG_OBJECT && val_clazz.is_array() {
                                true
                            } else {
                                ctx.vm.is_instance_of(val_clazz, target_clazz)
                            }
                        } else {
                            //error!(target: "validation", "Could not resolve both classes: {:?}, {:?}", val_clazz, target_clazz);
                            true
                        }
                    }
                } else {
                    false
                }
            }
            FieldType::Array(class_name, ..) => {
                if let Value::Reference(val_id) = value {
                    if val_id.is_null() {
                        true
                    } else {
                        /*let val_ref = ctx.vm.resolve_object_by_id(val_id)?;
                        &val_ref.class_name == class_name*/
                        // TODO
                        true
                    }
                } else {
                    false
                }
            }
            prim => prim == &value,
        } {
            Err(VmError::ValidationError(format!("[Validation]: Field and Value have incompatible types. Expected: {}, Got: {}", self.to_descriptor(), value.print(ctx.vm))))
        } else {
            Ok(())
        }
    }
}