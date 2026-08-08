use crate::vm::class::ClassAndMethodId;
use crate::vm::java_thread::{ThreadMeta, ThreadState};
use crate::vm::result::VMResult;
use crate::vm::value::RefId;
use crate::vm::{Context, VmError};
use parking_lot::{Mutex, RawMutex};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::thread::{park, park_timeout};
use std::time::Duration;
use parking_lot::lock_api::{MutexGuard};

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
    monitor_state: Mutex<MonitorState>,
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
            monitor_state: Mutex::new(state),
        }
    }
}

pub struct MonitorHandler {
    // TODO consider using RwLock again. because of mere holdsLock() check (enter can still lock as write)
    handler_state: Mutex<HashMap<MonitorAssociate, Arc<Monitor>>>,
}

impl MonitorHandler {
    pub fn new() -> Self {
        Self {
            handler_state: Mutex::new(HashMap::new()),
        }
    }

    fn get_monitor(&self, ctx: Context, associate: MonitorAssociate) -> Arc<Monitor> {
        let mut state = self.handler_state.lock();

        state
            .entry(associate)
            .or_insert_with(|| {
                Arc::new(Monitor::new(Arc::clone(&ctx.thread.meta), associate))
            })
            .clone()
    }

    pub fn enter_ref_or_block(&self, ctx: Context, ref_id: RefId) -> VMResult<()> {
        self.enter_or_block(ctx, MonitorAssociate::Ref(ref_id))
    }

    pub fn enter_method_or_block(&self, ctx: Context, method: ClassAndMethodId) -> VMResult<()> {
        self.enter_or_block(ctx, MonitorAssociate::Method(method))
    }

    fn enter_or_block(&self, ctx: Context, associate: MonitorAssociate) -> VMResult<()> {
        let monitor = self.get_monitor(ctx, associate);
        let mut monitor_guard = monitor.monitor_state.lock();

        if let Some(owner) = &monitor_guard.owner {
            if owner == &ctx.thread.meta {
                #[cfg(feature = "debug")]
                {
                    if monitor_guard.counter > 0 {
                        ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Reentered (at: {:?})", std::time::Instant::now()));
                    } else {
                        ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Entered (at: {:?})", std::time::Instant::now()));
                    }
                }

                // the monitor is owned by this thread -> enter again
                monitor_guard.counter += 1;
            } else {
                #[cfg(feature = "debug")]
                ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Enter blocked (at: {:?})", std::time::Instant::now()));
                // the monitor is currently held by another thread -> park
                monitor_guard.entry_list.push_back(Arc::clone(&ctx.thread.meta));
                drop(monitor_guard);
                ctx.thread.meta.block();
                park();
                ctx.check_canceled();
                #[cfg(feature = "debug")]
                ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Enter after unblock (at: {:?})", std::time::Instant::now()));
            }
        } else {
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Entered unowned (at: {:?})", std::time::Instant::now()));
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

    pub fn exit_method(&self, ctx: Context, method: ClassAndMethodId) -> VMResult<()> {
        self.exit(ctx, MonitorAssociate::Method(method))
    }

    fn exit(&self, ctx: Context, associate: MonitorAssociate) -> VMResult<()> {
        let monitor = {
            let state = self.handler_state.lock();

            state.get(&associate).ok_or_else(|| VmError::ValidationError(format!("Monitor for {:?} does not exist", associate))).cloned()
        }?;

        // has to own the monitor to exit
        let mut state_guard = monitor.monitor_state.lock();
        if !matches!(&state_guard.owner, Some(meta) if meta == &ctx.thread.meta) {
            return Err(VmError::ValidationError(format!("Cannot exit monitor owned by: {:?} with {:?}. (monitor: {:?})", &state_guard.owner, &ctx.thread.meta, state_guard.associate)))
        }

        if state_guard.counter > 1 {
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Exit one layer down (at: {:?})", std::time::Instant::now()));
            // entered multiple times -> decrease counter
            state_guard.counter -= 1;
        } else {
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Exit out (at: {:?})", std::time::Instant::now()));
            self.release_monitor(state_guard);
        }
        Ok(())
    }

    fn release_monitor(&self, mut state_guard: MutexGuard<RawMutex, MonitorState>) {
        // pass the ownership to next thread blocked on this, if any
        if let Some(to_wake) = state_guard.entry_list.pop_front() {
            state_guard.owner = Some(to_wake.clone());

            if let ThreadState::Notified(count) = *to_wake.thread_state.read() {
                state_guard.counter = count;
            } else {
                state_guard.counter = 1;
            }
            to_wake.unblock();
            to_wake.os_thread.unpark();
        } else {
            state_guard.owner = None;
            state_guard.counter = 0;
        }
    }

    pub fn holds_lock(&self, ctx: Context, ref_id: RefId) -> bool {
        let state_guard = self.handler_state.lock();
        if let Some(monitor) = state_guard.get(&MonitorAssociate::Ref(ref_id)) {
            let state_lock = monitor.monitor_state.lock();
            if let Some(owner) = &state_lock.owner {
                return owner == &ctx.thread.meta
            }
        }
        false
    }

    pub fn wait(&self, ctx: Context, ref_id: RefId, timeout: u64) -> VMResult<()> {
        let associate = MonitorAssociate::Ref(ref_id);
        // acquire the monitor
        let monitor = self.get_monitor(ctx, associate);

        // push ourselves onto the wait list
        let counter = {
            let mut monitor_guard = monitor.monitor_state.lock();
            assert_eq!(monitor_guard.owner, Some(ctx.thread.meta.clone()));
            monitor_guard.wait_list.push_back(ctx.thread.meta.clone());

            // release the ownership so it can get notified
            let counter = monitor_guard.counter;
            self.release_monitor(monitor_guard);
            counter
        };

        #[cfg(feature = "debug")]
        ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Waiting on (at: {:?})", std::time::Instant::now()));

        // wait until notified or timeout
        if timeout == 0 {
            ctx.thread.meta.wait(counter);
            park();
            ctx.check_canceled();
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Got notified (at: {:?})", std::time::Instant::now()));
        } else {
            ctx.thread.meta.wait(counter);
            park_timeout(Duration::from_millis(timeout));
            ctx.check_canceled();
            assert_eq!(1, counter);
            let current_state = ctx.thread.meta.thread_state.read().clone();
            match current_state {
                ThreadState::Waiting(_) => ctx.thread.meta.notified(),
                ThreadState::Running => {}
                other => unreachable!("{:?}", other)
            }
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(associate, format!("Got notified or timeout over (at: {:?})", std::time::Instant::now()));
            {
                let mut monitor_guard = monitor.monitor_state.lock();
                // unknown if unparked by timeout running out or notify. just to be safe
                monitor_guard.wait_list.retain(|m| m != &ctx.thread.meta);
                // this covers two cases when the timeout ran out before notify:
                // 1: the monitor currently has no owner -> acquire it immediately
                // 2: the monitor is currently held by another thread -> block until it can be acquired
                if monitor_guard.owner.clone().is_none_or(|o| o != ctx.thread.meta) {
                    drop(monitor_guard);
                    self.enter_or_block(ctx, associate)?;
                }
            }

        }
        // reached here when acquired ownership back through exit (because notify puts this on entry_list) or above acquiring
        Ok(())
    }

    pub fn notify(&self, ctx: Context, ref_id: RefId) -> VMResult<()>{
        let monitor = self.get_monitor(ctx, MonitorAssociate::Ref(ref_id));

        // has to own the monitor to notify
        let mut monitor_guard = monitor.monitor_state.lock();
        if monitor_guard.owner.clone().is_none_or(|o| o != ctx.thread.meta) {
            return Err(VmError::ValidationError(format!("IllegalMonitorStateException: owner is: {:?}", monitor_guard.owner)))
        }
        // set the waiting thread up for competing for the ownership
        if let Some(top) = monitor_guard.wait_list.pop_front() {
            top.notified();
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(monitor_guard.associate, format!("Notified {} (at: {:?})", top.id,  std::time::Instant::now()));
            monitor_guard.entry_list.push_back(top);
        }
        Ok(())
    }

    pub fn notify_all(&self, ctx: Context, ref_id: RefId) -> VMResult<()>{
        let monitor = self.get_monitor(ctx, MonitorAssociate::Ref(ref_id));

        // has to own the monitor to notify
        let mut monitor_guard = monitor.monitor_state.lock();
        if monitor_guard.owner.clone().is_none_or(|o| o != ctx.thread.meta) {
            return Err(VmError::ValidationError(format!("IllegalMonitorStateException: owner is: {:?}", monitor_guard.owner)))
        }
        // set the waiting threads up for competing for the ownership
        while let Some(top) = monitor_guard.wait_list.pop_front() {
            top.notified();
            #[cfg(feature = "debug")]
            ctx.thread.debug_helper.monitor_logger.push_event(monitor_guard.associate, format!("NotifyAlled {} (at: {:?})", top.id,  std::time::Instant::now()));
            monitor_guard.entry_list.push_back(top);
        }
        Ok(())
    }
}