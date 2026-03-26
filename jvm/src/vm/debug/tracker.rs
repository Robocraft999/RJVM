use log::{debug, info};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub struct Tracker{
    tracked_object_ids: HashSet<u32>,
    tracked_method_descs: HashSet<String>,
    logged_object_events: RefCell<HashMap<u32, Vec<String>>>,
    logged_method_events: RefCell<HashMap<String, Vec<String>>>,
}

impl Tracker {
    pub fn new(object_entries: Option<HashSet<u32>>, method_entries: Option<Vec<String>>) -> Self{
        let mut tracked_object_ids = HashSet::new();
        tracked_object_ids.extend(object_entries.unwrap_or_default());
        let mut tracked_method_descs = HashSet::new();
        tracked_method_descs.extend(method_entries.unwrap_or_default());
        #[cfg(feature = "debug")]
        {
            use crate::vm::debug::loader;
            let config = loader::load_config();
            if let Some(config) = config {
                tracked_object_ids.extend(config.tracker.ids);
                tracked_method_descs.extend(config.tracker.descs);
            }
        }
        Self{
            tracked_object_ids,
            tracked_method_descs,
            logged_object_events: RefCell::new(HashMap::new()),
            logged_method_events: RefCell::new(HashMap::new()),
        }
    }

    pub fn push_object_event(&self, id: u32, event: String){
        #[cfg(feature = "debug")]
        {
            if !self.tracked_object_ids.contains(&id) {
                return;
            }
            if self.logged_object_events.borrow().contains_key(&id){
                self.logged_object_events.borrow_mut().get_mut(&id).unwrap().push(event);
            } else {
                self.logged_object_events.borrow_mut().insert(id, vec![event]);
            }
        }
    }

    pub fn push_method_event(&self, sig: String, event: String) {
        #[cfg(feature = "debug")]
        {
            if !self.tracked_method_descs.contains(&sig){
                return;
            }
            if self.logged_method_events.borrow().contains_key(&sig){
                self.logged_method_events.borrow_mut().get_mut(&sig).unwrap().push(event);
            } else {
                self.logged_method_events.borrow_mut().insert(sig, vec![event]);
            }
        }
    }

    pub fn print(&self) {
        info!(target: "debug", "Object Tracker:");
        for (id, events) in self.logged_object_events.borrow().iter() {
            info!(target: "debug", "Events for: {}", id);
            for event in events.iter() {
                debug!(target: "debug", "  ~ {}", event);
            }
        }
        info!(target: "debug", "Method Tracker:");
        for (sig, events) in self.logged_method_events.borrow().iter() {
            info!(target: "debug", "Events for: {}", sig);
            for event in events.iter() {
                debug!(target: "debug", "  ~ {}", event);
            }
        }
    }
}