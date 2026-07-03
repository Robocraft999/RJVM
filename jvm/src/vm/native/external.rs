use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::class_file::methods::descriptor::MethodDescriptor;
use crate::vm::class::ClassAndMethod;
use crate::vm::jni::types::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort, jvalue, JNIEnv, JavaVM};
use crate::vm::value::{Reference, Value};
use libffi::high::CodePtr;
use libffi::middle::{Arg, Cif, Type};
use std::ffi::c_void;

fn primitive_type_to_native(primitive_type: &PrimitiveType) -> Type {
    match primitive_type {
        PrimitiveType::Boolean => Type::c_uchar(),
        PrimitiveType::Byte => Type::c_schar(),
        PrimitiveType::Char => Type::c_ushort(),
        PrimitiveType::Double => Type::f64(),
        PrimitiveType::Float => Type::f32(),
        PrimitiveType::Integer => Type::c_int(),
        PrimitiveType::Long => Type::c_long(),
        PrimitiveType::Short => Type::c_short(),
    }
}

fn field_type_to_native(field_type: &FieldType) -> Type{
    match field_type {
        FieldType::Object(..) => Type::pointer(),
        FieldType::Array(..) => Type::pointer(),
        FieldType::Primitive(pt) => primitive_type_to_native(pt)
    }
}

fn descriptor_to_cif(method_descriptor: &MethodDescriptor) -> Cif {
    // first arg is always JNIEnv, second arg is this / class depending on whether static
    let args: Vec<Type> = vec![Type::pointer(), primitive_type_to_native(&PrimitiveType::Integer)]
        .into_iter()
        .chain(method_descriptor.args.iter().map(field_type_to_native))
        .collect();
    let return_type = if let Some(ft) = &method_descriptor.return_type{
        field_type_to_native(ft)
    } else {
        Type::void()
    };
    Cif::new(args, return_type)
}

fn values_to_jni_args<'a>(args: &'a Vec<Value>) -> Vec<Arg<'a>> {
    args.iter().filter(|v| if let Value::Dummy = v {false} else {true}).map(|arg| {
        match arg{
            Value::Reference(reference) => Arg::new(&reference.0),
            Value::Integer(integer) => Arg::new(integer),
            Value::Long(long) => Arg::new(long),
            Value::Float(float) => Arg::new(float),
            Value::Double(double) => Arg::new(double),
            val => unreachable!("Value of type: {:?} cannot be converted to an arg", val)
        }
    }).collect()
}


#[derive(Debug, Clone)]
pub struct ExternNativeMethod{
    ptr: CodePtr,
    cif: Cif
}

unsafe impl Send for ExternNativeMethod {}

impl ExternNativeMethod {
    pub fn new(ptr: CodePtr, desc: &MethodDescriptor) -> Self{
        let cif = descriptor_to_cif(desc);
        Self { ptr, cif }
    }

    pub fn call<'a>(&self, java_vm: &JavaVM, class_and_method: &ClassAndMethod, object: Reference<'a>, args: Vec<Value>) -> Option<jvalue>{
        let env: *const JNIEnv = &java_vm.env;
        let second = object.id.nid() as jobject;
        let mut jni_args = vec![Arg::new(&env), Arg::new(&second)];
        jni_args.extend(values_to_jni_args(&args));
        unsafe {
            match class_and_method.method.descriptor.return_type{
                Some(FieldType::Object(..)) | Some(FieldType::Array(..)) => {
                    let object_id: jobject = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {l: object_id})
                }
                Some(FieldType::Primitive(PrimitiveType::Boolean)) => {
                    let val: jboolean = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {z: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Byte)) => {
                    let val: jbyte = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {b: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Char)) => {
                    let val: jchar = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {c: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Double)) => {
                    let val: jdouble = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {d: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Float)) => {
                    let val: jfloat = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {f: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Integer)) => {
                    let val: jint = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {i: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Long)) => {
                    let val: jlong = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {j: val})
                }
                Some(FieldType::Primitive(PrimitiveType::Short)) => {
                    let val: jshort = self.cif.call(self.ptr, jni_args.as_slice());
                    Some(jvalue {s: val})
                }
                None => {
                    let _: c_void = self.cif.call(self.ptr, jni_args.as_slice());
                    None
                }
            }
        }
    }
}