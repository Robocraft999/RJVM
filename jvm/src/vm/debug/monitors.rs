use std::cell::RefCell;
use std::collections::HashMap;
use log::{debug, info};
use crate::vm::monitoring::MonitorAssociate;

pub struct MonitorLogger {
    logged_events: RefCell<HashMap<MonitorAssociate, Vec<String>>>,
}

impl MonitorLogger {
    pub fn new() -> Self {
        Self {
            logged_events: RefCell::new(HashMap::new())
        }
    }

    pub fn push_event(&self, associate: MonitorAssociate, message: String) {
        #[cfg(feature = "debug")]
        {
            self.logged_events.borrow_mut()
                .entry(associate)
                .or_default()
                .push(message);
        }
    }

    pub fn print(&self) {
        info!(target: "debug", "Monitor Tracker:");
        for (id, events) in self.logged_events.borrow().iter() {
            info!(target: "debug", "Events for: {:?}", id);
            for event in events.iter() {
                debug!(target: "debug", "  ~ {}", event);
            }
        }
    }
}

