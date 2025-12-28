use crate::vm::debug::exceptions::ExceptionHelper;
use crate::vm::debug::tracker::Tracker;

mod exceptions;
mod tracker;

pub struct DebugHelper{
    pub exception_helper: ExceptionHelper,
    pub tracker: Tracker,
}

impl DebugHelper{
    pub fn new() -> Self{
        Self{
            exception_helper: ExceptionHelper::new(),
            tracker: Tracker::new(None)
        }
    }

    pub fn print(&self){
        #[cfg(feature = "debug")]
        {
            if let Some(config) = loader::load_config(){
                if config.enabled_modules.contains("exceptions"){ self.exception_helper.print() }
                if config.enabled_modules.contains("tracker"){ self.tracker.print() }
            } else {
                self.exception_helper.print();
                self.tracker.print();
            }
        }
    }
}


#[cfg(feature="debug")]
mod loader{
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::Read;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct Config{
        pub enabled_modules: HashSet<String>,
        pub tracker: TrackerConfig
    }

    #[derive(Deserialize, Debug)]
    pub struct TrackerConfig{
        pub ids: HashSet<u32>,
    }

    pub fn load_config() -> Option<Config>{
        let path = "resources/debug_config.toml";

        if let Ok(mut f) = File::open(path){
            let mut contents = String::new();
            if let Ok(_) = f.read_to_string(&mut contents){
                let config: Config = toml::from_str(&contents).unwrap();
                return Some(config);
            }
        }
        None
    }
}