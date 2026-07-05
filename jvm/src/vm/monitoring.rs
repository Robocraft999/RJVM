use std::collections::{HashMap, VecDeque};
use std::sync::{Arc};
use std::thread::park;
use parking_lot::{Mutex, RwLock};
use crate::vm::application::thread;
use crate::vm::class::ClassAndMethodId;
use crate::vm::{Context, VmError};
use crate::vm::java_thread::{JavaThread, ThreadMeta, TID};
use crate::vm::result::VMResult;
use crate::vm::value::RefId;

#[derive(Debug, PartialEq, Copy, Clone, Hash, Eq)]
pub enum MonitorAssociate {
    Ref(RefId),
    Method(ClassAndMethodId)
}

pub struct MonitorState {
    owner: Option<Arc<ThreadMeta>>,
    counter: usize,
    associate: MonitorAssociate,

    entry_list: VecDeque<Arc<ThreadMeta>>,
    wait_list: VecDeque<Arc<ThreadMeta>>,
}

pub struct Monitor {
    state: Mutex<MonitorState>,
}

impl Monitor {
    pub fn new(owner: Arc<ThreadMeta>, associate: MonitorAssociate) -> Self {
        let state = MonitorState {
            owner: Some(owner),
            counter: 0,
            associate,

            entry_list: VecDeque::new(),
            wait_list: VecDeque::new(),
        };
        Self {
            state: Mutex::new(state),
        }
    }
}

pub struct MonitorHandler {
    state: Mutex<HashMap<MonitorAssociate, Arc<Monitor>>>,
}

impl MonitorHandler {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
        }
    }

    pub fn enter_ref_or_block(&self, ctx: Context, ref_id: RefId) -> VMResult<()> {
        self.enter_or_block(ctx, MonitorAssociate::Ref(ref_id))
    }

    fn enter_or_block(&self, ctx: Context, associate: MonitorAssociate) -> VMResult<()> {
        let monitor = {
            let mut state = self.state.lock();

            state
                .entry(associate)
                .or_insert_with(|| {
                    Arc::new(Monitor::new(Arc::clone(&ctx.thread.meta), associate))
                })
                .clone()
        };
        let mut monitor_guard = monitor.state.lock();

        if let Some(owner) = &monitor_guard.owner {
            if owner == &ctx.thread.meta {
                // the monitor is owned by this thread -> enter again
                monitor_guard.counter += 1;
            } else {
                // the monitor is currently held by another thread -> park
                monitor_guard.entry_list.push_back(Arc::clone(&ctx.thread.meta));
                drop(monitor_guard);
                park();
            }
        } else {
            // monitor doesn't have an owner currently -> enter
            monitor_guard.owner = Some(Arc::clone(&ctx.thread.meta));
            // assert_eq!(0, monitor_guard.counter); // counter should be 0 at this point
            monitor_guard.counter = 1;
        }
        Ok(())
    }

    pub fn exit_ref(&self, ctx: Context, ref_id: RefId) -> VMResult<()> {
        self.exit(ctx, MonitorAssociate::Ref(ref_id))
    }

    fn exit(&self, ctx: Context, associate: MonitorAssociate) -> VMResult<()> {
        let monitor = {
            let state = self.state.lock();

            state.get(&associate).ok_or(VmError::ValidationError(format!("Monitor for {:?} does not exist", associate))).cloned()
        }?;

        let mut state_guard = monitor.state.lock();
        if !matches!(&state_guard.owner, Some(meta) if meta == &ctx.thread.meta) {
            return Err(VmError::ValidationError(format!("Cannot exit monitor owned by: {:?} with {:?}", &state_guard.owner, &ctx.thread.meta)))
        }

        if state_guard.counter > 1 {
            state_guard.counter -= 1;
        } else {
            if let Some(to_wake) = state_guard.entry_list.pop_front() {
                state_guard.owner = Some(to_wake.clone());
                state_guard.counter = 1;
                to_wake.os_thread.unpark();
            } else {
                state_guard.owner = None;
                state_guard.counter = 0;
            }
        }
        Ok(())
    }
}