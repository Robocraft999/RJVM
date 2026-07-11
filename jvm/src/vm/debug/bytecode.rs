use crate::vm::class::ClassRef;
use log::warn;
use serde::Deserialize;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

#[derive(Debug)]
#[cfg_attr(feature = "debug", derive(Deserialize))]
pub struct ClassFilter {
    name: String,
    methods: Option<Vec<(String, String)>>
}

impl PartialEq<ClassRef<'_>> for &ClassFilter {
    fn eq(&self, other: &ClassRef) -> bool {
        if self.name != other.name {
            return false;
        }
        let Some(methods) = &self.methods else { return true };
        methods.iter().map(|(name, desc)| other.find_method(name, desc)).try_collect::<Vec<_>>().is_some()
    }
}

pub struct BytecodeHelper {
    tracked_classes: Vec<ClassFilter>,
    cached_bytecode: RefCell<Vec<(String, Vec<u8>)>>,
}

impl BytecodeHelper {
    pub fn new() -> Self {
        let mut tracked_classes = Vec::new();
        #[cfg(feature = "debug")]
        {
            use crate::vm::debug::loader;
            let config = loader::load_config();
            if let Some(config) = config {
                tracked_classes.extend(config.bytecode.classes);
            }
        }
        Self {
            tracked_classes,
            cached_bytecode: RefCell::new(Vec::new()),
        }
    }

    pub fn push_class(&self, class: ClassRef, bytecode: Vec<u8>) {
        #[cfg(feature = "debug")]
        {
            if self.tracked_classes.iter().find(|f| f == &class).is_some() {
                self.cached_bytecode.borrow_mut().push((class.name.clone(), bytecode));
            }
        }
    }

    pub fn print(&self) {
        for (i, (name, bytecode)) in self.cached_bytecode.borrow().iter().enumerate() {
            let path = format!("resources/debug/bytecode/{}_{}.class", i, name.replace("/", "."));
            //let path = "resources/debug/bytecode/dump.class";
            println!("{}", path.as_str());
            let mut dump_file = File::options().write(true).create(true).open(path.as_str()).unwrap();

            if dump_file.write_all(bytecode).is_err() {
                warn!("Failed to write to dump file for class: {}", name);
            }

            dump_file.flush().unwrap();
        }
    }
}