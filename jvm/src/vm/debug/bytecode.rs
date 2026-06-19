use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use log::warn;

pub struct BytecodeHelper {
    tracked_classes: HashSet<String>,
    cached_bytecode: RefCell<Vec<(String, Vec<u8>)>>,
}

impl BytecodeHelper {
    pub fn new() -> Self {
        let mut tracked_classes = HashSet::new();
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

    pub fn push_class(&self, class_name: &str, bytecode: Vec<u8>) {
        #[cfg(feature = "debug")]
        {
            if self.tracked_classes.contains(class_name) {
                self.cached_bytecode.borrow_mut().push((class_name.to_owned(), bytecode));
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