use crate::access_flags::FieldFlags;
use crate::attribute::{Attribute, ConstantValue};
use crate::vm::result::VMResult;
use crate::vm::value::Value;
use crate::vm::VmError;
use lazy_regex::{lazy_regex, regex, Lazy};
use regex::Regex;
use std::str::FromStr;
use crate::method_info::MethodDescriptor;

#[derive(Debug)]
pub struct FieldInfo{
    pub flags: FieldFlags,
    pub name: String,
    pub field_type: FieldType,
    pub deprecated: bool,
    pub constant_value: Option<ConstantValue>,
    pub attributes: Vec<Attribute>
}

static PATTERN: Lazy<Regex> = lazy_regex!(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[/a-zA-Z$0-9_]+);)");

pub fn get_field_type_raw_parts(raw: &str) -> VMResult<(Option<&str>, Option<&str>, Option<&str>)> {
    if let Some(cap) = PATTERN.captures(raw){
        Ok((cap.name("object").map(|m| m.as_str()), cap.name("primitive").map(|m| m.as_str()), cap.name("array").map(|m| m.as_str())))
    } else {
        Err(VmError::ValidationError(format!("{} is not a valid field type", raw)))
    }
}

pub fn extract_component_type_from_array_class(array_class_descriptor: &str) -> VMResult<(FieldType, usize)> {
    let (object, primitive, array) = get_field_type_raw_parts(array_class_descriptor)?;
    let array_type = FieldType::from_raw_parts(object, primitive, array)?;
    if let FieldType::Array(_, component_type) = array_type {
        Ok((*component_type, array.unwrap_or("").len()))
    } else {
        Err(VmError::ValidationError("Can't extract component type from non-array type".to_string()))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType{
    Primitive(PrimitiveType),
    Object(String),
    Array(String, Box<FieldType>),
}

impl FieldType {
    pub(crate) fn to_class_name(&self) -> String {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type_to_class_name(primitive_type),
            FieldType::Object(name) => format!("{}", name),
            FieldType::Array(name, _) => format!("{}", name),
        }
    }

    pub fn to_descriptor(&self) -> String {
        match self {
            FieldType::Primitive(primitive_type) => primitive_type_to_descriptor(primitive_type),
            FieldType::Object(name) => format!("L{};", name),
            FieldType::Array(name, _) => format!("{}", name),
        }
    }

    pub fn to_array_field_type(self, dims: usize) -> FieldType{
        if dims == 0{
            panic!("Can't make {self:?} an array type because dims is 0");
        }
        let prefix = "[".repeat(dims);
        match self.clone() {
            FieldType::Primitive(primitive_type) => {
                let name = prefix + primitive_type_to_descriptor(&primitive_type).as_str();
                FieldType::Array(name, Box::new(self))
            }
            FieldType::Object(name) => {
                let name = prefix + "L" + name.as_str() + ";";
                FieldType::Array(name, Box::new(self))
            }
            //FIXME should we allow this?
            FieldType::Array(_, _) => panic!("Can't make {self:?} an array type, because it is already one"),
        }
    }

    pub fn get_locals_length(&self) -> usize{
        match self{
            FieldType::Object(_) => 1,
            FieldType::Array(_, _) => 1,
            FieldType::Primitive(PrimitiveType::Boolean) => 1,
            FieldType::Primitive(PrimitiveType::Byte) => 1,
            FieldType::Primitive(PrimitiveType::Char) => 1,
            FieldType::Primitive(PrimitiveType::Short) => 1,
            FieldType::Primitive(PrimitiveType::Integer) => 1,
            FieldType::Primitive(PrimitiveType::Long) => 2,
            FieldType::Primitive(PrimitiveType::Float) => 1,
            FieldType::Primitive(PrimitiveType::Double) => 2,
        }
    }

    pub fn get_default_value<'a>(&self, null: Value<'a>) -> Value<'a> {
        match self {
            FieldType::Primitive(primitive) => {
                match primitive {
                    PrimitiveType::Boolean => Value::Integer(0),
                    PrimitiveType::Byte => Value::Integer(0),
                    PrimitiveType::Char => Value::Integer(0),
                    PrimitiveType::Short => Value::Integer(0),
                    PrimitiveType::Integer => Value::Integer(0),
                    PrimitiveType::Long => Value::Long(0),
                    PrimitiveType::Float => Value::Float(0f32),
                    PrimitiveType::Double => Value::Double(0f64),
                }
            }
            FieldType::Object(_) => null,
            FieldType::Array(_, _) => null,
        }
    }

    pub fn from_raw_parts(object: Option<&str>, primitive: Option<&str>, array: Option<&str>) -> VMResult<FieldType>{
        let field_type = if let Some(obj) = object{
            Some(FieldType::Object(String::from(obj)))
        }else if let Some(prim) = primitive{
            Some(FieldType::Primitive(PrimitiveType::from_str(prim)?))
        } else {
            None
        };
        if let Some(dims_amount_of_brackets) = array{
            let mut name = String::new();
            name.push_str(dims_amount_of_brackets);
            if let Some(obj) = object{
                name.push_str("L");
                name.push_str(obj);
                name.push_str(";");
            }
            if let Some(prim) = primitive{
                name.push_str(prim);
            }
            let component_type = field_type.ok_or(VmError::ValidationError(format!("{} is neither object nor primitive field type", name)))?;
            Ok(FieldType::Array(name, Box::from(component_type)))
        } else {
            field_type.ok_or(VmError::ValidationError("Field type is neither object nor primitive".to_string()))
        }
    }
}

impl FromStr for FieldType{
    type Err = VmError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (object, primitive, array) = get_field_type_raw_parts(s)?;
        FieldType::from_raw_parts(object, primitive, array)
    }
}

impl PartialEq<Value<'_>> for FieldType{
    fn eq(&self, other: &Value) -> bool {
        match (other, self) {
            (Value::Reference(..), FieldType::Object(..)) | (Value::Reference(..), FieldType::Array(..)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Integer)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Short)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Byte)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Boolean)) => true,
            (Value::Integer(..), FieldType::Primitive(PrimitiveType::Char)) => true,
            (Value::Long(..), FieldType::Primitive(PrimitiveType::Long)) => true,
            (Value::Float(..), FieldType::Primitive(PrimitiveType::Float)) => true,
            (Value::Double(..), FieldType::Primitive(PrimitiveType::Double)) => true,
            _ => false
        }
    }
}

pub fn get_primitive_class(short_name: &str) -> String{
    match short_name {
        "Z" => "java/lang/Boolean",
        "B" => "java/lang/Byte",
        "C" => "java/lang/Character",
        "S" => "java/lang/Short",
        "I" => "java/lang/Integer",
        "J" => "java/lang/Long",
        "F" => "java/lang/Float",
        "D" => "java/lang/Double",
        _   => unreachable!("Type is not primitive")
    }.to_string()
}

pub fn primitive_to_wrapper_name(prim_name: &str) -> String{
    match prim_name {
        "boolean" => "java/lang/Boolean",
        "byte" => "java/lang/Byte",
        "char" => "java/lang/Character",
        "short" => "java/lang/Short",
        "int" => "java/lang/Integer",
        "long" => "java/lang/Long",
        "float" => "java/lang/Float",
        "double" => "java/lang/Double",
        "void" => "java/lang/Void",
        _ => unreachable!("Type is not primitive")
    }.to_string()
}

pub fn primitive_type_to_class_name(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::Integer => "java/lang/Integer".to_string(),
        PrimitiveType::Long    => "java/lang/Long".to_string(),
        PrimitiveType::Short   => "java/lang/Short".to_string(),
        PrimitiveType::Char    => "java/lang/Character".to_string(),
        PrimitiveType::Byte    => "java/lang/Byte".to_string(),
        PrimitiveType::Float   => "java/lang/Float".to_string(),
        PrimitiveType::Double  => "java/lang/Double".to_string(),
        PrimitiveType::Boolean => "java/lang/Boolean".to_string(),
    }
}

pub fn primitive_type_to_descriptor(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::Boolean => String::from("Z"),
        PrimitiveType::Byte => String::from("B"),
        PrimitiveType::Char => String::from("C"),
        PrimitiveType::Double => String::from("D"),
        PrimitiveType::Float => String::from("F"),
        PrimitiveType::Integer => String::from("I"),
        PrimitiveType::Long => String::from("J"),
        PrimitiveType::Short => String::from("S"),
    }
}

/**
* class_name without L{class_name};
*/
pub fn get_class_descriptor(class_name: &str) -> String{
    match class_name {
        "java/lang/Boolean" => "Z".to_string(),
        "java/lang/Byte" => "B".to_string(),
        "java/lang/Character" => "C".to_string(),
        "java/lang/Short" => "S".to_string(),
        "java/lang/Integer" => "I".to_string(),
        "java/lang/Long" => "J".to_string(),
        "java/lang/Float" => "F".to_string(),
        "java/lang/Double" => "D".to_string(),
        _ => "L".to_string() + class_name + ";"
    }
}

pub fn native_escape(name: &str) -> String {
    let mut escaped = String::new();
    for c in name.chars(){
        match c{
            'A'..='Z' | 'a'..='z' | '0'..='9' => escaped.push(c),
            '/' => escaped.push('_'),
            '_' => escaped.push_str("_1"),
            ';' => escaped.push_str("_2"),
            '[' => escaped.push_str("_3"),
            other => escaped.push_str(format!("_0{:04x}", other as u16).as_str()),
        }
    }
    escaped
}

pub fn native_escaped_descriptor(descriptor: &MethodDescriptor) -> String {
    let mut escaped = String::new();
    for ft in descriptor.args.iter().chain(descriptor.return_type.iter()) {
        escaped.push_str(native_escape(ft.to_descriptor().as_str()).as_str());
    }
    escaped
}

#[derive(Debug, PartialEq, Clone)]
pub enum PrimitiveType{
    Boolean,
    Byte,
    Char,
    Double,
    Float,
    Integer,
    Long,
    Short,
}

impl FromStr for PrimitiveType{
    type Err = VmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Z" => Ok(Self::Boolean),
            "B" => Ok(Self::Byte),
            "S" => Ok(Self::Short),
            "I" => Ok(Self::Integer),
            "J" => Ok(Self::Long),
            "F" => Ok(Self::Float),
            "D" => Ok(Self::Double),
            "C" => Ok(Self::Char),
            _ => Err(VmError::ValidationError(format!("Invalid primitive type {}", s)))
        }
    }
}

#[cfg(test)]
mod tests{
    use crate::field_info::{native_escape, native_escaped_descriptor};
    use crate::method_info::MethodDescriptor;

    #[test]
    fn test_native_escape() {
        assert_eq!("", native_escape(""));
        assert_eq!("A", native_escape("A"));
        assert_eq!("hello_test_Test123", native_escape("hello/test/Test123"));
        assert_eq!("hello_Pr_000fcfer", native_escape("hello/Prüfer"))
    }

    #[test]
    fn test_native_escape_descriptor(){
        let descriptor = MethodDescriptor::new(String::from("(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;I)Ljava/lang/Object;"));
        assert_eq!("Ljava_lang_reflect_Constructor_2_3Ljava_lang_Object_2ILjava_lang_Object_2", native_escaped_descriptor(&descriptor));
    }

    fn class_and_method_escaped(class_name: &str, method_name: &str, descriptor: &MethodDescriptor) -> (String, String) {
        let mut short = String::from("Java_");
        short.push_str(native_escape(class_name).as_str());
        short.push('_');
        short.push_str(native_escape(method_name).as_str());

        let mut long = short.clone();
        long.push_str("__");
        long.push_str(native_escaped_descriptor(&descriptor).as_str());

        (short, long)
    }

    #[test]
    fn test_native_escaped_class_and_method_1(){
        let class_name = "sun/reflect/NativeConstructorAccessorImpl";
        let method_name = "newInstance0";
        let descriptor = MethodDescriptor::new(String::from("(Ljava/lang/reflect/Constructor;[Ljava/lang/Object;I)Ljava/lang/Object;"));
        let expected = (
            String::from("Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0"),
            String::from("Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0__Ljava_lang_reflect_Constructor_2_3Ljava_lang_Object_2ILjava_lang_Object_2")
        );
        assert_eq!(expected, class_and_method_escaped(class_name, method_name, &descriptor));
    }
    #[test]
    fn test_native_escaped_class_and_method_2(){
        //sun/awt/X11GraphicsEnvironment.getNumScreens()I
        let class_name = "sun/awt/X11GraphicsEnvironment";
        let method_name = "getNumScreens";
        let descriptor = MethodDescriptor::new(String::from("()I"));
        let expected = (
            String::from("Java_sun_awt_X11GraphicsEnvironment_getNumScreens"),
            String::from("Java_sun_awt_X11GraphicsEnvironment_getNumScreens__I")
        );
        assert_eq!(expected, class_and_method_escaped(class_name, method_name, &descriptor));
    }
}