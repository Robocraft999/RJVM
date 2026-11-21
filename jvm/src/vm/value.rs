use crate::vm::class::ClassId;
use crate::vm::constants::STRING_value_INDEX;
use crate::vm::heap::array::ArrayContent;
use crate::vm::jni::types::jobject;
use crate::vm::result::VMResult;
use crate::vm::{VM, VmError};
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
            Self::Reference(rv) => {
                if rv.is_null(){
                    write!(f, "VNull")
                } else {
                    write!(f,"{:?}", rv)
                }
            },
            Self::Uninitialized => write!(f, "VUninitialized"),
            Self::Integer(value) => write!(f, "VInt ({})", value),
            Self::Long(value) => write!(f, "VLong ({})", value),
            Self::Float(value) => write!(f, "VFloat ({:.8})", value),
            Self::Double(value) => write!(f, "VDouble ({:.8})", value),
            Self::Dummy => write!(f, "VDummy")
        }
    }
}

impl Value{
    pub fn print(&self, vm: &VM) -> String {
        match self {
            Self::Reference(id) => {
                if id.is_null(){
                    "VNull".to_string()
                } else {
                    let rv = vm.resolve_object_by_id(*id).unwrap();
                    format!("{:?}", rv.print(vm))
                }
            },
            Self::Uninitialized => "VUninitialized".to_string(),
            Self::Integer(value) => format!("VInt ({})", value),
            Self::Long(value) => format!("VLong ({})", value),
            Self::Float(value) => format!("VFloat ({:.8})", value),
            Self::Double(value) => format!("VDouble ({:.8})", value),
            Self::Dummy => "VDummy".to_string(),
        }
    }

    pub fn expect_int(&self) -> Result<i32, VmError> {
        if let Self::Integer(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected integer but found {:?}", self)))
        }
    }

    pub fn expect_long(&self) -> Result<i64, VmError> {
        if let Self::Long(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected long but found {:?}", self)))
        }
    }

    pub fn expect_float(&self) -> Result<f32, VmError> {
        if let Self::Float(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected float but found {:?}", self)))
        }
    }

    pub fn expect_double(&self) -> Result<f64, VmError> {
        if let Self::Double(value) = self{
            Ok(*value)
        } else {
            Err(VmError::ValidationError(format!("Expected double but found {:?}", self)))
        }
    }
    
    pub const fn get_computational_type(&self) -> i32{
        match self {
            Self::Uninitialized => -1,
            Self::Reference(_) => 1,
            Self::Integer(_) => 1,
            Self::Long(_) => 2,
            Self::Float(_) => 1,
            Self::Double(_) => 2,
            Self::Dummy => -1,
        }
    }

    pub const fn is_null(&self) -> bool{
        if let Self::Reference(r) = self{
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
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
    pub const fn nid(&self) -> jobject { self.0 as jobject }
}

macro_rules! gen_typed_get_field {
    ($name:ident, $typ:ident, $res_type:ty) => {
        pub fn $name(&self, index: usize) -> VMResult<$res_type> {
            match &self.reference_type {
                ReferenceType::Object(fields) => {
                    let field = fields.read()[index];
                    if let Value::$typ(inner) = field {
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
            ReferenceType::Array(..) => {unimplemented!("This reference represents an array, please use 'set_element()'")}
        };
    }

    pub fn get_field(&self, index: usize) -> Value{
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                fields.read()[index]
            }
            ReferenceType::Array(..) => {unimplemented!("This reference represents an array, please use 'get_element()'")}
        }
    }

    gen_typed_get_field!(get_ref_field, Reference, RefId);
    gen_typed_get_field!(get_int_field, Integer, i32);
    gen_typed_get_field!(get_long_field, Long, i64);
    gen_typed_get_field!(get_float_field, Float, f32);
    gen_typed_get_field!(get_double_field, Double, f64);

    pub fn set_element(&self, index: usize, value: Value) {
        match &self.reference_type {
            ReferenceType::Object(..) => {unimplemented!("This reference represents an object, please use 'set_field()'")}
            ReferenceType::Array(content) => {
                content.write().set(index, value)
            }
        };
    }

    pub fn get_element(&self, index: usize) -> Value{
        match &self.reference_type {
            ReferenceType::Object(..) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(content) => {
                content.read().get(index).unwrap()
            }
        }
    }

    pub fn get_length(&self) -> usize{
        match &self.reference_type {
            ReferenceType::Object(..) => {unimplemented!("This reference represents an object, please use 'get_field()'")}
            ReferenceType::Array(content) => {
                content.read().len()
            }
        }
    }

    pub const fn is_array(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(..) => true,
            ReferenceType::Object(..) => false
        }
    }

    pub const fn is_object(&self) -> bool{
        match self.reference_type {
            ReferenceType::Array(..) => false,
            ReferenceType::Object(..) => true
        }
    }

    pub const fn is_null(&self) -> bool{
        self.id.is_null()
    }

    pub fn print<'a>(&self, vm: &VM<'a>) -> String {
        format!("VRef [ id: {:?}, class: {}:{}, type: {}, components: [{}] ]",
                self.id.0,
                self.class_name, self.class_id.0,
                if matches!(self.reference_type, ReferenceType::Object(..)) { "Object" } else { "Array" },
                self.get_components_printable(vm).join(", ")
        )
    }

    fn get_components_printable<'a>(&self, vm: &VM<'a>) -> Vec<String>{
        let object = |field: &Value| match field {
            Value::Reference(id) => {
                if id.is_null() { return "Null".to_owned(); }
                let rv = vm.resolve_object_by_id(*id).unwrap();
                if rv.class_name == "java/lang/String" {
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_string_from_ref(rv).unwrap_or_else(|_| "VMError".to_owned()))
                } else if rv.class_name == "[C"{
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_string_from_char_arr(*field).unwrap_or_else(|_| "VMError".to_owned()))
                } else if rv.class_name == "java/lang/Class" {
                    format!("{:?}:{}:{:?}->'{}'", rv.id.0, rv.class_name, rv.class_id.0, vm.extract_class_name_from_class_ref(rv).unwrap_or_else(|_| "VMError".to_owned()))
                } else {
                    format!("{:?}:{}:{:?}", rv.id.0, rv.class_name, rv.class_id.0)
                }
            }
            _ => format!("{:?}", field)
        };
        match &self.reference_type {
            ReferenceType::Object(fields) => {
                if self.class_name == "java/lang/String" {
                    let internal = vm.extract_string_from_char_arr(self.get_field(STRING_value_INDEX)).unwrap_or_else(|_| "VMError".to_owned());
                    let mut components = Vec::new();
                    components.push(internal);
                    let mut other_fields = fields.read().iter().skip(1).map(object).collect();
                    components.append(&mut other_fields);
                    components
                } else {
                    fields.read().iter().map(object).collect()
                }
            },
            ReferenceType::Array(content) => {
                //vec![String::from("<redacted>")]
                let content = content.read();
                if content.is_char() {
                    vec![content.get_as_string().unwrap()]
                } else if content.is_byte() {
                    content.as_vec().iter().map(|e| if let Value::Integer(val) = e {format!("{:02x}", val)} else {format!("{e:?}")}).collect()
                } else {
                    let mut vec = Vec::new();
                    let mut null_counter = 0;
                    for value in content.as_vec().iter(){
                        if value.is_null() {
                            null_counter += 1;
                        } else {
                            if null_counter > 0{
                                vec.push(format!("{}xNull", null_counter));
                                null_counter = 0;
                            }
                            vec.push(object(value));
                        }
                    }
                    drop(content);
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
    Array(RwLock<ArrayContent>)
}