use crate::class_file::fields::field_type::FieldType;
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct MethodDescriptor{
    raw: String,
    pub args: Vec<FieldType>,
    pub return_type: Option<FieldType>,
}

impl Hash for MethodDescriptor{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

static PATTERN: Lazy<Regex> = lazy_regex!(r"(?<array>\[+)?(?:(?<primitive>[ZBSIJFDC])|L(?<object>[\/a-zA-Z$0-9_]+);|(?<void>V))");

impl MethodDescriptor{
    pub fn new(raw_string: String) -> Self{
        let mut args = Vec::new();
        let mut void_return = false;
        for cap in PATTERN.captures_iter(raw_string.as_str()){
            if cap.name("void").is_some() {
                void_return = true;
                continue
            }
            let object = cap.name("object").map(|m| m.as_str());
            let primitive = cap.name("primitive").map(|m| m.as_str());
            let array = cap.name("array").map(|m| m.as_str());
            //FIXME error handling
            args.push(FieldType::from_raw_parts(object, primitive, array).unwrap());
        }

        let return_type = if void_return {None} else {args.pop()};

        Self{
            raw: raw_string,
            args,
            return_type,
        }
    }

    pub fn matches(&self, other: &str) -> bool{
        //TODO maybe parse other or do it better in some way
        self.raw == other
    }

    pub fn as_str(&self) -> &str{
        self.raw.as_str()
    }
}

impl PartialEq for MethodDescriptor{
    fn eq(&self, other: &Self) -> bool {
        self.matches(other.raw.as_str())
    }
}

impl Eq for MethodDescriptor{}