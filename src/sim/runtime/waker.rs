use std::{cell::RefCell, rc::Weak, sync::Arc};

use futures::task::ArcWake;

use super::{state::State, task::TaskId};

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct Waker {
    pub state: Weak<RefCell<State>>,
    pub task_id: TaskId,
}

// Waker will not be send between threads by design
unsafe impl Send for Waker {}
unsafe impl Sync for Waker {}

impl ArcWake for Waker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let Some(state) = arc_self.state.upgrade() else {
            return;
        };
        state.borrow_mut().push_task(arc_self.task_id);
    }
}
