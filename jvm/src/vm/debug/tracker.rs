use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use log::{debug, info};

pub struct Tracker{
    tracked_object_ids: HashSet<u32>,
    logged_events: RefCell<HashMap<u32, Vec<String>>>,
}

impl Tracker {
    pub fn new(entries: Option<HashSet<u32>>) -> Self{
        let mut tracked_object_ids = HashSet::new();
        tracked_object_ids.extend(entries.unwrap_or_default());
        #[cfg(feature = "debug")]
        {
            use crate::vm::debug::loader;
            let config = loader::load_config();
            if let Some(config) = config {
            tracked_object_ids.extend(config.tracker.ids)
            }
        }
        Self{
            tracked_object_ids,
            logged_events: RefCell::new(HashMap::new()),
        }
    }

    pub fn push_event(&self, id: u32, event: String){
        #[cfg(feature = "debug")]
        {
            if !self.tracked_object_ids.contains(&id) {
                return;
            }
            if self.logged_events.borrow().contains_key(&id){
                self.logged_events.borrow_mut().get_mut(&id).unwrap().push(event);
            } else {
                self.logged_events.borrow_mut().insert(id, vec![event]);
            }
        }
    }

    pub fn print(&self) {
        for (id, events) in self.logged_events.borrow().iter() {
            info!(target: "debug", "Events for: {}", id);
            for event in events.iter() {
                debug!(target: "debug", "  ~ {}", event);
            }
        }
    }
}