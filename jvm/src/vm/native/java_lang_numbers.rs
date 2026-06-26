use crate::vm::constants::classes::{JAVA_LANG_DOUBLE, JAVA_LANG_FLOAT};
use crate::vm::jni::types::JavaVM;
use crate::vm::native::{gen_delegate, invalidation, non_failing_some, NativeMethodRegistry};
use crate::vm::result::VMPartialResult;
use crate::vm::value::{Reference, Value};
use crate::vm::{VmError, VM};

pub fn register_natives(registry: &mut NativeMethodRegistry) {
    registry.register(JAVA_LANG_FLOAT, "floatToRawIntBits", "(F)I", delegate_float_to_raw_bits);
    registry.register(JAVA_LANG_DOUBLE, "doubleToRawLongBits", "(D)J", delegate_double_to_raw_bits);
    registry.register(JAVA_LANG_DOUBLE, "longBitsToDouble", "(J)D", delegate_long_bits_to_double);
}

gen_delegate!(delegate_float_to_raw_bits, |_ctx, _obj_ref, args| {
    if let Some(Value::Float(value)) = args.get(0){
        non_failing_some(Value::Integer(value.to_bits() as i32))
    } else {
        invalidation!("Expected float but got: {:?}", args.get(0))
    }
});

gen_delegate!(delegate_double_to_raw_bits, |_ctx, _obj_ref, args| {
    if let Some(Value::Double(value)) = args.get(0){
        non_failing_some(Value::Long(value.to_bits() as i64))
    } else {
        invalidation!("Expected double but got: {:?}", args.get(0))
    }
});

gen_delegate!(delegate_long_bits_to_double, |_ctx, _obj_ref, args| {
    if let Some(Value::Long(value)) = args.get(0){
        non_failing_some(Value::Double(f64::from_bits(*value as u64)))
    } else {
        invalidation!("Expected long but got: {:?}", args.get(0))
    }
});