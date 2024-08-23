use core::cell::RefCell;
use std::{future::Future, rc::Weak};

use crate::sim::runtime::JoinHandle;

use super::{state::NodeState, Node};

////////////////////////////////////////////////////////////////////////////////

thread_local! {
    static NODE_HANDLE: RefCell<Option<NodeHandle>> = const { RefCell::new(None) };
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct NodeHandle(pub(crate) Weak<NodeState>);

impl NodeHandle {
    pub fn current() -> Self {
        NODE_HANDLE.with(|h| {
            h.borrow()
                .as_ref()
                .expect("node handle can be obtained within a simulation only")
                .clone()
        })
    }

    pub fn set(handle: Option<NodeHandle>) {
        NODE_HANDLE.with(|h| *h.borrow_mut() = handle);
    }

    fn get_state(&self) -> Node {
        Node(self.0.upgrade().unwrap())
    }

    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.get_state().0.runtime.spawn(task)
    }

    pub fn next_step(&self) -> bool {
        Self::set(Some(self.clone()));
        let result = self.get_state().0.runtime.next_step();
        Self::set(None);
        result
    }

    pub fn make_steps(&self, steps: Option<usize>) -> usize {
        Self::set(Some(self.clone()));
        let result = self.get_state().0.runtime.make_steps(steps);
        Self::set(None);
        result
    }
}
