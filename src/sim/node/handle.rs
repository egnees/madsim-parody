use core::cell::RefCell;
use std::{future::Future, rc::Weak, time::Duration};

use crate::sim::{runtime::JoinHandle, time::Timestamp};

use super::{guard::ContextGuard, state::NodeState, Node};

////////////////////////////////////////////////////////////////////////////////

thread_local! {
    static NODE_HANDLE: RefCell<Option<NodeHandle>> = const { RefCell::new(None) };
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct NodeHandle(pub(crate) Weak<NodeState>);

impl NodeHandle {
    pub(crate) fn exists() -> bool {
        NODE_HANDLE.with(|h| h.borrow().is_some())
    }

    pub fn current() -> Self {
        NODE_HANDLE.with(|h| {
            h.borrow()
                .as_ref()
                .expect("node handle can be obtained only within a simulation")
                .clone()
        })
    }

    pub(crate) fn set(handle: Option<NodeHandle>) {
        NODE_HANDLE.with(|h| *h.borrow_mut() = handle);
    }

    pub(crate) fn get_state(&self) -> Node {
        Node(self.0.upgrade().unwrap())
    }

    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.get_state().0.runtime.spawn(task)
    }

    pub fn next_step(&self) -> bool {
        let _guard = ContextGuard::new(self.clone());
        let state = self.get_state().0;
        let runtime_made_step = state.runtime.next_step();
        if runtime_made_step {
            true
        } else {
            state.time_handle.0.borrow_mut().advance_to_next_timer()
        }
    }

    pub fn make_steps(&self, steps: Option<usize>) -> usize {
        let mut made_steps = 0;
        if let Some(steps) = steps {
            for _ in 0..steps {
                if !self.next_step() {
                    break;
                }
                made_steps += 1;
            }
        } else {
            while self.next_step() {
                made_steps += 1;
            }
        }
        made_steps
    }

    pub fn time(&self) -> Timestamp {
        self.get_state().0.time_handle.0.borrow().time()
    }

    fn next_event_timestamp(&self) -> Option<Timestamp> {
        let state = self.get_state().0;
        if state.runtime.has_work() {
            Some(self.time())
        } else {
            state
                .time_handle
                .0
                .borrow()
                .next_timer()
                .map(|entry| entry.timestamp)
        }
    }

    pub fn step_duration(&self, duration: Duration) -> usize {
        let until = self.time() + duration;
        let mut steps = 0;
        while let Some(next) = self.next_event_timestamp() {
            if next <= until {
                self.next_step();
                steps += 1
            } else {
                break;
            }
        }
        assert!(self.time() <= until);
        self.get_state()
            .0
            .time_handle
            .0
            .borrow_mut()
            .advance_time(until);
        steps
    }
}
