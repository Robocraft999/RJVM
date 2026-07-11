use crate::class_file::fields::field_type::{FieldType, PrimitiveType};
use crate::vm::class::ClassId;
use crate::vm::constants::STRING_value_INDEX;
use crate::vm::jni::types::jobject;
use crate::vm::result::VMResult;
use crate::vm::{VmError, VM};
use parking_lot::RwLock;
use std::fmt::{Debug, Formatter};

#[derive(PartialEq, Default, Clone, Copy)]
pub enum Value{
    #[default]
    Uninitialized,
    Reference(RefId),

    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Dummy,
}

impl Debug for Value{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Reference(rv) => {
                if rv.is_null(){
                    write!(f, "VNull")
                } else {
                    write!(f,"{:?}", rv)
                }
            },
            Value::Uninitialized => write!(f, "VUninitialized"),
            Value::Integer(value) => write!(f, "VInt ({})", value),
            Value::Long(value) => write!(f, "VLong ({})", value),
            Value::Float(value) => write!(f, "VFloat ({:.8})", value),
            Value::Double(value) => write!(f, "VDouble ({:.8})", value),
            Value::Dummy => write!(f, "VDummy")
        }
    }
}

impl Value{
    pub fn print(&self, vm: &VM) -> String {
        match self {
            Value::Reference(id) => {
                if id.is_null(){
                    "VNull".to_string()
                } else {
                    let rv = vm.resolve_object_by_id(*id).unwrap();
                    format!("{:?}", rv.print(vm))
                }
            },
            Value::Uninitialized => "VUninitialized".to_string(),
            Value::Integer(value) => format!("VInt ({})", value),
            Value::Long(value) => format!("VLong ({})", value),
            Value::Float(value) => format!("VFloat ({:.8})", value),
            Value::Double(value) => format!("VDouble ({:.8})", value),
            Value::Dummy => "VDummy".to_string(),
        }
    }

    pub fn expect_int(&self) -> Result<i32, VmError> {
        if let Value::Integer(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected integer but found {:?}", self)))
        }
    }

    pub fn expect_long(&self) -> Result<i64, VmError> {
        if let Value::Long(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected long but found {:?}", self)))
        }
    }

    pub fn expect_float(&self) -> Result<f32, VmError> {
        if let Value::Float(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected float but found {:?}", self)))
        }
    }

    pub fn expect_double(&self) -> Result<f64, VmError> {
        if let Value::Double(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected double but found {:?}", self)))
        }
    }
    
    pub fn get_computational_type(&self) -> i32{
        match self {
            Value::Uninitialized => -1,
            Value::Reference(_) => 1,
            Value::Integer(_) => 1,
            Value::Long(_) => 2,
            Value::Float(_) => 1,
            Value::Double(_) => 2,
            Value::Dummy => -1,
        }
    }

    pub fn is_null(&self) -> bool{
        if let Value::Reference(r) = self{
            r.is_null()
        } else {
            false
        }
    }
}

impl From<bool> for Value{
    fn from(value: bool) -> Self{
        Self::Integer(if value { 1 } else { 0 })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RefId(pub u32);

impl RefId {
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
    pub fn nid(&self) -> jobject { self.0 as jobject }
}

macro_rules! gen_typed_get_field {
    ($name:ident, $typ:ident, $res_type:ty) => {
        pub fn $name(&self, index: usize) -> VMResult<$res_type> {
            match &self.reference_type {
                ReferenceType::Object(fields) => {
                    if let Value::$typ(inner) = fields.read()[index] {
                        Ok(inner)
                    } else {
                        Err(VmError::ValidationError(format!("Expected {} field at '{}'", stringify!($typ), index)))
                    }
                }
                ReferenceType::Array(..) => Err(VmError::ValidationError("This reference represents an array, please use 'get_element()'".to_string())),
            }
        }
    };
}

pub type Reference<'a> = &'a ReferenceValue;

pub struct ReferenceValue{
    pub(crate) id: RefId,
    pub(crate) class_id: ClassId,
    pub(crate) class_name: String,
    pub(crate) reference_type: ReferenceType,
}

impl ReferenceValue{
    //FIXME switch these to Option for safety
    pub fn set_field(&self, index: usize, value: Value) {
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                fields.write()[index] = value
            }
            ReferenceType::Array(_, _, _) => {unimplemented!("This reference represents an array, please use 'set_element()'")}
        };
    }

    pub fn get_field(&self, index: usize) -> Value{
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                fields.read()[index].clone()
            }
            ReferenceType::Array(_, _, _) => {unimplemented!("This reference represents an array, please use 'get_element()'")}
        }
    }

    gen_typed_get_field!(get_ref_field, Reference, RefId);
    gen_typed_get_field!(get_int_field, Integer, i32);
    gen_typed_get_field!(get_long_field, Long, i64);

    pub fn set_element(&self, index: usize, value: Value) {
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'set_field()'")}
            ReferenceType::Array(_, _, content) => {
                content.write()[index] = value
            }
        };
    }

    pub fn get_element(&self, index: usize) -> Value{
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(_, _, content) => {
                content.read()[index].clone()
            }
        }
    }

    pub fn get_length(&self) -> usize{
        match &self.reference_type {
            ReferenceType::Object(_) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(_, _, content) => {
                content.read().len()
            }
        }
    }

    pub fn is_array(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(_, _, _) => true,
            ReferenceType::Object(_) => false
        }
    }

    pub fn is_object(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(_, _, _) => false,
            ReferenceType::Object(_) => true
        }
    }

    pub fn is_null(&self) -> bool{
        self.id.is_null()
    }

    pub fn print<'a>(&self, vm: &VM<'a>) -> String {
        format!("VRef [ id: {:?}, class: {}:{}, type: {}, components: [{}] ]",
                self.id.0,
                &self.class_name, &self.class_id.0,
                if matches!(self.reference_type, ReferenceType::Object(..)) { "Object" } else { "Array" },
                self.get_components_printable(vm).join(", ")
        )
    }

    fn get_components_printable<'a>(&self, vm: &VM<'a>) -> Vec<String>{
        let object = |field: &Value| match field {
            Value::Reference(id) => {
                if id.is_null() { return "Null".to_string(); }
                let rv = vm.resolve_object_by_id(*id).unwrap();
                if rv.class_name == "java/lang/String" {
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_string_from_ref(rv).unwrap_or("VMError".to_string()))
                } else if rv.class_name == "[C"{
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_string_from_char_arr(field.clone()).unwrap_or("VMError".to_string()))
                } else if rv.class_name == "java/lang/Class" {
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_class_name_from_class_ref(rv).unwrap_or("VMError".to_string()))
                } else {
                    format!("{:?}:{}:{:?}", rv.id.0, rv.class_name, rv.class_id.0)
                }
            }
            _ => format!("{:?}", field)
        };
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                if self.class_name == "java/lang/String" {
                    let internal = vm.extract_string_from_char_arr(self.get_field(STRING_value_INDEX)).unwrap_or("VMError".to_string());
                    let mut components = Vec::new();
                    components.push(internal);
                    let mut other_fields = fields.read().iter().skip(1).map(object).collect();
                    components.append(&mut other_fields);
                    components
                } else {
                    fields.read().iter().map(object).collect()
                }
            },
            ReferenceType::Array(_, field_type, content) => {
                //vec![String::from("<redacted>")]
                if let FieldType::Primitive(PrimitiveType::Char) = field_type {
                    let chars: Vec<char> = content.read().iter().map(|e| if let Value::Integer(val) = e {char::from_u32(*val as u32).unwrap()} else {'?'}).collect();
                    vec![chars.iter().collect::<String>()]
                } else if let FieldType::Primitive(PrimitiveType::Byte) = field_type {
                    content.read().iter().map(|e| if let Value::Integer(val) = e {format!("{:02x}", val)} else {format!("{e:?}")}).collect()
                } else {
                    let mut vec = Vec::new();
                    let mut null_counter = 0;
                    for value in content.read().iter(){
                        if value.is_null() {
                            null_counter += 1;
                        } else {
                            if null_counter > 0{
                                vec.push(format!("{}xNull", null_counter));
                                null_counter = 0;
                            }
                            vec.push(object(&value));
                        }
                    }
                    if null_counter > 0{
                        vec.push(format!("{}xNull", null_counter));
                    }
                    vec
                }
            }
        }
    }
}

impl PartialEq for ReferenceValue {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/*impl Debug for ReferenceValue{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VRef")
            .field("object_id", &self.id)
            .field("class", &format_args!("{}:{}", &self.class_name, &self.class_id.0))
            .field("type", &match self.reference_type {
                ReferenceType::Object(_) => "Object",
                ReferenceType::Array(_, _, _) => "Array",
            })
            .field("components", &self.get_components_printable())
            .finish()
    }
}*/

impl Debug for ReferenceValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<VRef>")
    }
}

pub enum ReferenceType{
    Object(RwLock<Vec<Value>>),
    Array(usize, FieldType, RwLock<Vec<Value>>)
}